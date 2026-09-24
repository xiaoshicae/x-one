package xgorm

import (
	"cmp"
	"context"
	"database/sql"
	"errors"
	"io"
	"log/slog"
	"strings"
	"time"

	mysqldriver "github.com/go-sql-driver/mysql"
	"github.com/jackc/pgx/v5/pgconn"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"

	"github.com/xiaoshicae/x-one/internal/xclient"
	"github.com/xiaoshicae/x-one/xconfig"
	"github.com/xiaoshicae/x-one/xerror"
	"github.com/xiaoshicae/x-one/xhook"
	"github.com/xiaoshicae/x-one/xmetric"
	"github.com/xiaoshicae/x-one/xutil"
)

const (
	// pingAttempts 建连验证的尝试次数
	pingAttempts = 3

	// fallbackPingTimeout 配置里推算不出预算时，单次 Ping 的兜底超时
	fallbackPingTimeout = time.Second
)

// pingInterval 第一次退避的上界，之后逐次翻倍（xutil.Retry），3 次尝试之间的两次
// 退避上界是 1s、2s。这个数写在 docs/config.md「建连重试」里，启动预算靠它推算。
//
// 是变量而不是常量，只为让测试能调短——连不上的用例要跑满整轮重试，
// 按一秒算一次就是几十秒。
var pingInterval = time.Second

// New 按配置建一个 GORM 实例，不触碰任何全局变量。
//
// ctx 限定建连验证的生命期：地址不通时这里要走满一轮 Ping 重试，
// 收到退出信号就该当场放弃，而不是让进程卡在一个注定连不上的库上。
//
// 返回的 io.Closer 关闭底层连接池。建连失败时不会留下连接池。
func New(ctx context.Context, cfg ClientConfig) (*gorm.DB, io.Closer, error) {
	if err := cfg.validate(); err != nil {
		return nil, nil, xerror.Newf("xgorm", "config", "invalid config: %w", err)
	}

	dsn, info, err := resolveDSN(cfg)
	if err != nil {
		return nil, nil, xerror.New("xgorm", "config", err)
	}

	dialect, _ := lookupDialect(cfg.Driver) // validate 已经确认它注册过
	dialector := dialect.Open(dsn)          // Logger 要看它的占位符长什么样，所以先造出来

	// 关掉 GORM 自带的那次 ping：它用的是自己的 context，我们的退出信号
	// 和重试都管不到它。开着的话，连一个不可达的地址时 New 会先在里面
	// 干等满 DSN 的 connect_timeout，哪怕 ctx 早就被取消了（实测 3 秒）。
	// 关掉之后 gorm.Open 只做装配、立刻返回，全部建连都走下面那次
	// ctx-aware 的 ping —— 取消得了、也重试得了。
	//
	// 内置的 MySQL 和 ClickHouse 方言在 Initialize 里各有一次查版本，也会建连，
	// 两者都在 Open 里关掉、挪进了 Dialect.Ready（见 openMySQL）。
	gormCfg := &gorm.Config{DisableAutomaticPing: true}
	if cfg.Log {
		gormCfg.Logger = newGormLogger(cfg, dialector)
	} else {
		// 不给 Logger 的话 GORM 会补上自己的默认实现，而那个默认实现
		// 是「带 ANSI 颜色地往 os.Stdout 写」：慢 SQL 和执行错误照样打，
		// 只是绕开了 slog——没有级别、没有 TraceID、不是 JSON，
		// 一行彩色文本直接插进日志流里。Log=false 要的是不打，不是换个地方打
		gormCfg.Logger = logger.Discard
	}

	db, err := gorm.Open(dialector, gormCfg)
	if err != nil {
		// 不用替它收拾：gorm.Open 自己失败时会把它建的池子关掉
		// （gorm v1.31.2 gorm.go 的 Open，Initialize 失败与自动 ping 失败两条路都关）
		//
		// 内置方言走不到这里的认证失败：它们的 Open 不碰网络。注册进来的方言要是
		// 在 Initialize 里建连，密码错就出在这里，照样要说清是认证失败
		if authFailed(err) {
			return nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)
		}
		return nil, nil, xerror.Newf("xgorm", "connect", "open %s failed: %w", info.Addr, err)
	}
	pool, err := db.DB()
	if err != nil {
		return nil, nil, xerror.Newf("xgorm", "connect", "get underlying pool: %w", err)
	}

	// 从这里往后的每一步都是我们自己的，失败了没人替我们收拾：
	// 连接池已经活着，不关就漏一个常驻协程
	ok := false
	defer func() {
		if !ok {
			if cerr := pool.Close(); cerr != nil {
				slog.Warn("xgorm failed to close the pool", "error", cerr)
			}
		}
	}()
	pool.SetMaxOpenConns(cfg.MaxOpenConns)
	pool.SetMaxIdleConns(cfg.MaxIdleConns)
	pool.SetConnMaxLifetime(cfg.MaxLifetime)
	pool.SetConnMaxIdleTime(cfg.MaxIdleTime)

	if err := ping(ctx, pool, pingTimeout(cfg, info), readyOf(dialect, db)); err != nil {
		if authFailed(err) {
			return nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)
		}
		return nil, nil, xerror.Newf("xgorm", "connect", "cannot reach %s: %w", info.Addr, err)
	}

	if cfg.Trace {
		if err := installTracing(db, info); err != nil {
			return nil, nil, xerror.Newf("xgorm", "new", "install tracing callbacks: %w", err)
		}
	}

	logConn(info, cfg)

	ok = true
	return db, &poolCloser{pool: pool, info: info}, nil
}

// ping 建连验证，失败按退避重试；ctx 取消时立即放弃。
//
// ready 非空时在每次 Ping 成功之后接着执行，失败同样算这一次没通过、同样重试，
// 见 Dialect.Ready。
//
// 认证失败不重试：服务端已经明确拒绝了这组凭证，再试两次只是把同一个错误
// 多等两轮退避（默认最多 3s）才报出来，还会在服务端多留两条认证失败的记录。
// Retry 没有「别再试了」的出口，所以取消这一轮的 ctx 让它停下，报的仍是认证那个错误。
func ping(ctx context.Context, pool *sql.DB, timeout time.Duration, ready func(context.Context) error) error {
	ctx, stop := context.WithCancel(ctx)
	defer stop()
	var denied error
	err := xutil.Retry(ctx, pingAttempts, timeout, pingInterval, func(ctx context.Context) error {
		err := pool.PingContext(ctx)
		if err == nil && ready != nil {
			err = ready(ctx)
		}
		if authFailed(err) {
			denied = err
			stop()
		}
		return err
	})
	if denied != nil {
		return denied
	}
	return err
}

// authFailed 服务端是否拒绝了这组凭证。
//
// PostgreSQL：SQLSTATE 第 28 类（invalid authorization specification）。
// 实测 pgx v5.10.0 连 PG 16，密码错、用户不存在都报 28P01（后者在 scram 认证下
// 也是 "password authentication failed"），错误链上是 *pgconn.PgError。
// 库不存在是 3D000，不在这一类里，仍按连不上处理。
//
// MySQL：错误号 1045 和 1044，错误链上是 *mysql.MySQLError。实测 go-sql-driver v1.10.1
// 连 MySQL 8.0.46：密码错、用户不存在都是 1045（Access denied for user …）；
// 账号对、但没有这个库的权限是 1044（Access denied … to database …）——
// 库不存在时，没有全局权限的账号拿到的也是 1044 而不是 1049，服务端不告诉它库在不在。
// 不看 SQLSTATE：1045 的是 28000，1044 的却是 42000（语法错误、权限错误共用的那一类）。
func authFailed(err error) bool {
	var pgErr *pgconn.PgError
	if errors.As(err, &pgErr) {
		return strings.HasPrefix(pgErr.Code, "28")
	}
	var myErr *mysqldriver.MySQLError
	return errors.As(err, &myErr) && (myErr.Number == mysqlAccessDenied || myErr.Number == mysqlDBAccessDenied)
}

// MySQL 的两个认证错误号（ER_ACCESS_DENIED_ERROR、ER_DBACCESS_DENIED_ERROR）
const (
	mysqlAccessDenied   = 1045
	mysqlDBAccessDenied = 1044
)

// readyOf 把方言的 Ready 绑到这个实例上，方言没提供就是 nil
func readyOf(d Dialect, db *gorm.DB) func(context.Context) error {
	if d.Ready == nil {
		return nil
	}
	return func(ctx context.Context) error { return d.Ready(ctx, db) }
}

// pingTimeout 单次探测的超时
//
// 不能只用建连超时：探测的耗时是建连加一个往返，
// 拿建连预算当整体预算，连接刚建成就会被判超时。
//
// 超时以驱动从最终 DSN 里读出来的为准（info，见 ConnInfo.DialTimeout），
// 读不出来才用配置：配置里的只是注入 DSN 的默认值，使用者在 DSN 里写了
// connect_timeout=10 的话驱动就会等 10s，这里按配置的 500ms 算预算，
// 就会在驱动自己放弃之前把一次慢一点但合法的建连判超时、重试。
func pingTimeout(cfg ClientConfig, info ConnInfo) time.Duration {
	dial := cmp.Or(info.DialTimeout, cfg.DialTimeout)
	var d time.Duration
	switch cfg.Driver {
	case DriverMySQL:
		// timeout 管建连，readTimeout 管 Ping 那个往返
		d = dial + cmp.Or(info.ReadTimeout, cfg.MySQL.ReadTimeout)
	case DriverPostgres:
		// 注入的 connect_timeout 是向上取整的整秒（默认 500ms 注进去是 1s），
		// 而且 pgx 拿它管的是每个主机的整个建连：TCP 之后的 TLS 握手、startup、
		// 认证都在里面（pgconn v5.10.0 "restricts the whole connection process"；
		// 实测 TCP 秒连、startup 不回话的服务端，connect_timeout=1 等满 1.0s）。
		// 预算只给 DialTimeout 的话，一次慢一点但合法的握手会在 pgx 自己放弃
		// 之前就被这里判超时。再加一份 DialTimeout 给 Ping 本身那个往返
		d = cmp.Or(info.DialTimeout, ceilSeconds(cfg.DialTimeout)) + cfg.DialTimeout
	default:
		// 其余驱动（如 ClickHouse 的 dial_timeout）同样拿建连超时管握手，
		// 往返没有单独的配置可依，按同一量级再给一份
		d = 2 * dial
	}
	if d <= 0 {
		return fallbackPingTimeout
	}
	return d
}

type poolCloser struct {
	pool *sql.DB
	info ConnInfo
}

func (c *poolCloser) Close() error {
	if err := c.pool.Close(); err != nil {
		return xerror.Newf("xgorm", "close", "close %s failed: %w", c.info.Addr, err)
	}
	return nil
}

// ---- 全局实例 ----

// C 取一个 GORM 实例，不带参数时取名为 default 的那个。
//
// 取不到直接 panic，理由见 xclient.Registry.Get：返回 nil 只会把同一个 panic
// 推迟到调用方第一次用它的时候，而那里看不出根因是配置没配。
//
// 可选依赖（配了就用、没配就跳过）用 Has 先判断。
func C(name ...string) *gorm.DB { return reg.Get(name...).db }

// CWithCtx 取实例并绑定 ctx，链路和超时才能传到下游。
//
// GORM 必须这样传 ctx（不像 go-redis 每个方法都收 ctx），所以这个方法是必要的。
func CWithCtx(ctx context.Context, name ...string) *gorm.DB {
	return C(name...).WithContext(ctx)
}

// Has 报告指定实例是否已配置，供可选依赖判断
func Has(name ...string) bool { return reg.Has(name...) }

// Names 返回已配置的实例名
func Names() []string { return reg.Names() }

// ---- 登记 ----

// instance 一个实例连同它自己的 Metric 开关。
//
// 开关跟着实例走：连接池 collector 是进程级的一个，抓取时遍历全部实例，
// 不带着这一位的话它分不出哪个实例配了 Metric: false，只能全导或全不导。
type instance struct {
	db     *gorm.DB
	metric bool
}

// reg 具名实例注册表。取实例、找不到时的报错、关闭时摘干净，
// 这些语义在 xgorm / xredis / xcache 之间必须一致，所以共用一份实现。
var reg = xclient.NewRegistry[instance]("xgorm", ConfigKey)

func init() {
	xhook.BeforeStart(initXGorm, xhook.At(xhook.StageClient))
	xhook.BeforeStop(closeXGorm, xhook.At(xhook.StageClient))

	// go-sql-driver 的日志是进程级的一个，只能在这里接，理由同 xredis：New 不碰全局，
	// 启动钩子里接又会盖掉使用者在 main 里自己调的 mysql.SetLogger。
	// 它不读配置、不做 IO，写的是一个包级变量，和登记钩子是同一类事。
	// 驱动在解析 DSN 时把当时的 logger 抄进连接配置，之后再调 SetLogger 只对新解析的 DSN 生效
	// ——xgorm 在启动钩子里才解析，所以 main 里、xone 启动之前调的那次照样生效
	_ = mysqldriver.SetLogger(mysqlDriverLogger{}) // 只在参数为 nil 时报错
}

// initXGorm 读配置，按名字把实例挨个建出来。
//
// 没配这一块就一个都不建：xgorm 是可选依赖，没配不该让服务起不来。
// 但 Build 照样走一遍，注册表由此知道启动钩子跑过了——之后 C() 取不到时
// 报的是「没配」，而不是「调早了」。
func initXGorm(ctx context.Context) error {
	if !xconfig.Has(ConfigKey) {
		return xclient.Build(ctx, reg, nil, build)
	}
	c, err := loadConfig()
	if err != nil {
		return err
	}
	return install(ctx, c)
}

// install 按配置把实例挨个建出来
func install(ctx context.Context, c Config) error {
	if err := xclient.Build(ctx, reg, c.Clients, build); err != nil {
		return err
	}
	installPoolMetrics() // 一个实例都没开 Metric 时它什么都不导出，不必特判
	slog.Info("xgorm ready", "instances", reg.Names())
	return nil
}

// closeXGorm 摘掉全部实例并逆序关闭。
//
// 启动钩子失败时它不会被调到——停止钩子只在和它配对的启动钩子成功之后才执行，
// 所以这里不必处理「还没建起来」。
func closeXGorm(context.Context) error { return reg.Close() }

// build 建一个实例。包一层 New 而不是直接把 New 交出去，
// 是为了把这个实例的 Metric 开关一起带进注册表
func build(ctx context.Context, c ClientConfig) (instance, io.Closer, error) {
	db, closer, err := New(ctx, c)
	if err != nil {
		return instance{}, nil, err
	}
	return instance{db: db, metric: c.Metric}, closer, nil
}

// installPoolMetrics 把连接池 collector 挂到当前的 Registry 上。
//
// 每次 install 都挂，不用 sync.Once：xmetric 重新装一次（同一进程里走第二轮
// 生命周期）就是一个新的 Registry，Once 挡住的正是这第二次，连接池指标
// 从此一个都导不出去。同一个 Registry 上重复挂由 xmetric.Register 认出来、
// 复用已有的那个，不会报重复注册。
//
// xmetric 在 StageTelemetry 就绪，早于这里；collector 抓取时才遍历实例。
//
// 不让启动失败：指标导不出去是可观测性问题，不该拦住服务起来。
func installPoolMetrics() {
	if _, err := xmetric.RegisterAs(newPoolCollector(xmetric.Namespace(), xmetric.ConstLabels(), poolStats)); err != nil {
		slog.Error("xgorm failed to register pool metrics", "error", err)
	}
}

// poolStats 读开了 Metric 的各实例连接池的实时状态
func poolStats() map[string]sql.DBStats {
	out := map[string]sql.DBStats{}
	for _, name := range reg.Names() {
		inst, ok := reg.Lookup(name)
		if !ok || !inst.metric {
			continue
		}
		pool, err := inst.db.DB()
		if err != nil || pool == nil {
			continue
		}
		out[name] = pool.Stats()
	}
	return out
}
