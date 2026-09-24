package xgorm

import (
	"context"
	"fmt"
	"log/slog"
	"regexp"
	"slices"
	"strconv"
	"strings"

	"gorm.io/driver/mysql"
	"gorm.io/gorm"
	"gorm.io/gorm/callbacks"
)

// openMySQL 造 MySQL 的 Dialector，并关掉它在 Initialize 里那次查版本。
//
// 那次 SELECT VERSION() 用的是写死的 context.Background()
// （gorm.io/driver/mysql v1.6.0 mysql.go:130），而且就在 gorm.Open 里、
// 在 xgorm 带重试的建连验证之前——它才是第一次建连。实测（MySQL 8.0.46，
// 对端收下连接却不回话）：只建了 1 个连接，3.0s（= 握手那一读等满 ReadTimeout）
// 后 gorm.Open 直接报 invalid connection，三次重试一次都没轮上；启动期间的
// 退出信号也管不到它，信号之后 2.95s 进程才退。
//
// 版本号本身还要：驱动靠它决定 MariaDB / MySQL 5.x 上改不改得了索引名和列名、
// 支不支持 FOR SHARE、DROP CONSTRAINT，以及 MariaDB 10.5+ 上用不用 RETURNING。
// 所以这次查询挪到 probeMySQLVersion，由建连验证来做：受 ctx 管，跟着重试。
// 关掉之后 gorm.Open 只做 sql.Open 和装配，不碰网络。
func openMySQL(dsn string) gorm.Dialector {
	return mysql.New(mysql.Config{DSN: dsn, SkipInitializeWithVersion: true})
}

// probeMySQLVersion 查服务端版本，按驱动自己的规则设好版本相关的开关。
//
// 作为 MySQL 方言的 Ready，在每次 Ping 成功之后执行，见 Dialect.Ready。
func probeMySQLVersion(ctx context.Context, db *gorm.DB) error {
	d, ok := db.Dialector.(*mysql.Dialector)
	if !ok {
		return nil // 不是我们造的 Dialector，没有开关可设
	}
	var v string
	if err := db.ConnPool.QueryRowContext(ctx, "SELECT VERSION()").Scan(&v); err != nil {
		return err
	}
	return applyMySQLVersion(db, d.Config, v)
}

// applyMySQLVersion 按版本号设老版本不支持的那几项。
//
// 规则抄自 gorm.io/driver/mysql v1.6.0 的 Initialize，升级驱动时要对一遍。
func applyMySQLVersion(db *gorm.DB, c *mysql.Config, v string) error {
	c.ServerVersion = v
	withReturning := false
	switch {
	case strings.Contains(v, "MariaDB"):
		c.DontSupportRenameIndex = true
		c.DontSupportRenameColumn = true
		c.DontSupportForShareClause = true
		c.DontSupportNullAsDefaultValue = true
		withReturning = versionAtLeast(v, "10.5")
	case strings.HasPrefix(v, "5.6."):
		c.DontSupportRenameIndex = true
		c.DontSupportRenameColumn = true
		c.DontSupportForShareClause = true
		c.DontSupportDropConstraint = true
	case strings.HasPrefix(v, "5.7."):
		c.DontSupportRenameColumn = true
		c.DontSupportForShareClause = true
		c.DontSupportDropConstraint = true
	case strings.HasPrefix(v, "5."):
		c.DisableDatetimePrecision = true
		c.DontSupportRenameIndex = true
		c.DontSupportRenameColumn = true
		c.DontSupportForShareClause = true
		c.DontSupportDropConstraint = true
	}
	if strings.Contains(v, "TiDB") {
		c.DontSupportRenameColumnUnique = true
	}
	if withReturning && !c.DisableWithReturning {
		return enableReturning(db)
	}
	return nil
}

// enableReturning 让增删改带上 RETURNING。
//
// 驱动在 Initialize 里注册回调时就把「支不支持 RETURNING」闭包进了
// gorm:create / gorm:update / gorm:delete（gorm v1.31.2 callbacks/create.go:38），
// 那时版本还没查，注册的是不支持的那一份。这里按支持的配置换掉那三个回调，
// 子句列表与驱动在 Initialize 里拼的一致。
func enableReturning(db *gorm.DB) error {
	cfg := &callbacks.Config{
		CreateClauses: append(slices.Clone(mysql.CreateClauses), "RETURNING"),
		UpdateClauses: append(slices.Clone(mysql.UpdateClauses), "RETURNING"),
		DeleteClauses: append(slices.Clone(mysql.DeleteClauses), "RETURNING"),
	}
	cb := db.Callback()
	if err := cb.Create().Replace("gorm:create", callbacks.Create(cfg)); err != nil {
		return err
	}
	if err := cb.Update().Replace("gorm:update", callbacks.Update(cfg)); err != nil {
		return err
	}
	if err := cb.Delete().Replace("gorm:delete", callbacks.Delete(cfg)); err != nil {
		return err
	}
	cb.Create().Clauses = cfg.CreateClauses
	cb.Update().Clauses = cfg.UpdateClauses
	cb.Delete().Clauses = cfg.DeleteClauses
	return nil
}

// versionAtLeast v 是否不低于 floor，逐段比较数字部分。抄自驱动的 checkVersion
func versionAtLeast(v, floor string) bool {
	if v == floor {
		return true
	}
	vs, ms := strings.Split(v, "."), strings.Split(floor, ".")
	for i, s := range vs {
		if i >= len(ms) {
			return true
		}
		a, b := leadingInt(s), leadingInt(ms[i])
		if a != b {
			return a > b
		}
	}
	return false
}

var leadingDigits = regexp.MustCompile(`^\d+`)

func leadingInt(s string) int {
	n, _ := strconv.Atoi(leadingDigits.FindString(s))
	return n
}

// mysqlDriverLogger 把 go-sql-driver 自己的日志接到 slog 上。
//
// go-sql-driver v1.10.1 的默认 logger 是 log.New(os.Stderr, "[mysql] ", …)，
// 往 stderr 写纯文本（`[mysql] 2026/09/24 10:00:00 packets.go:58 read tcp …: i/o timeout`），
// 不进 slog 就不是 JSON，进不了日志平台。它的日志没有级别，写出来的都是出错时的补充
// （读包失败、关掉坏连接、认证插件回退），调用方同时也拿到了错误，所以一律记成 WARN。
// 驱动不给 ctx，这些日志不带 trace_id。
type mysqlDriverLogger struct{}

func (mysqlDriverLogger) Print(v ...any) {
	slog.Warn("xgorm go-sql-driver log", "detail", strings.TrimSpace(fmt.Sprint(v...)))
}
