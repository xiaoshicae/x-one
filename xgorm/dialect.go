package xgorm

import (
	"context"
	"fmt"
	"slices"
	"sync"
	"time"

	"gorm.io/gorm"
)

// ConnInfo 可以安全写进日志的连接信息
//
// 从 DSN 里解出来的、确定不含凭证的那几项。本包不打印 DSN，
// 所以也就没有什么需要脱敏——理由见 dsn.go 开头。
type ConnInfo struct {
	Driver string // 驱动名，如 mysql
	Addr   string // 主机:端口
	DB     string // 库名

	// DialTimeout / ReadTimeout 驱动从最终 DSN 里读出来的建连、读超时，
	// 0 表示没写或读不出来。xgorm 拿它们推算建连验证时单次探测的预算：
	// 使用者在 DSN 里写了比配置更长的超时，预算要跟着放宽，
	// 不能在驱动自己放弃之前就把一次慢一点但合法的建连判超时
	DialTimeout time.Duration
	ReadTimeout time.Duration
}

// Dialect 一种数据库方言。
//
// mysql 和 postgres 内置。其余驱动由独立的 module 提供，在自己的 init 里
// 注册进来，使用者匿名 import 即可：
//
//	import _ "github.com/xiaoshicae/x-one/xgorm/clickhouse"
//
// 拆成独立 module 是因为驱动很重。实测一个只 import xgorm 的应用模块图是
// 65 个，加上 ClickHouse 驱动变成 146 个（编译包 140 → 183）——多出来的
// 大头是 Docker 和 testcontainers，因为 clickhouse-go 把集成测试用的它们
// 写在了自己 go.mod 的主 require 块里，而 go.mod 分不出「只测试用」。
// Go 的 MVS 正是按模块图把版本要求强加给使用者的，哪怕他只用 MySQL、
// 一个 ClickHouse 的包都没 import。
type Dialect struct {
	// Name 驱动名，即配置里 Driver 那一项要写的值
	Name Driver

	// Open 造 GORM 的 Dialector
	Open func(dsn string) gorm.Dialector

	// Resolve 把配置里的超时等参数注入 DSN，并解出可安全记录的连接信息。
	//
	// 留空表示 DSN 原样使用、连接信息里只有驱动名——对一个只想先跑起来的
	// 驱动这是合理的起点，超时写进 DSN 里一样有效。
	// 连接信息里填上 DialTimeout / ReadTimeout 的话，建连验证的预算会跟着
	// DSN 里实际生效的超时走；不填就只按配置推算。
	//
	// 返回普通 error 即可，由 xgorm 在模块边界包成一层 xerror（op 为 config）。
	// DSN 解不出来时别回传解析器的原始错误：那里面带着 DSN 片段，
	// 而错误信息会被记下来。
	Resolve func(c ClientConfig) (dsn string, info ConnInfo, err error)

	// Ready 建连验证通过之后调用，给驱动一个受 ctx 管的地方做它自己的初始化查询。
	// 可以留空。
	//
	// 有的 Dialector 在 Initialize 里查一次服务端版本，用的是写死的
	// context.Background()：退出信号管不到它，失败了也轮不到重试，
	// 因为它发生在 gorm.Open 里、在 xgorm 的建连验证之前。这类驱动在 Open 里
	// 把那次查询关掉、挪到这里：它和每次 Ping 一起执行、一起重试，受 ctx 约束。
	// 返回普通 error，xgorm 会包成 op 为 connect 的 xerror。
	Ready func(ctx context.Context, db *gorm.DB) error
}

// dialects 已注册的方言
//
// 加锁而不是裸 map：只在各包 init 里注册的话确实用不着——init 全部跑完、
// 单协程，之后 main 才开始读。但 RegisterDialect 是导出的，没人拦着在运行时调它，
// 而 New 可能正在别的协程里查表；map 的并发读写是会直接崩的那一种错。
var (
	dialectMu sync.RWMutex
	dialects  = map[Driver]Dialect{}
)

// RegisterDialect 注册一个驱动。
//
// 同名重复注册直接 panic：两个 Dialect 抢同一个名字时，选哪个都可能让服务
// 连到一个它以为自己没在连的地方。这是 import 期的问题，不该留到运行时。
func RegisterDialect(d Dialect) {
	if d.Name == "" {
		panic("xgorm: Dialect.Name must not be empty")
	}
	if d.Open == nil {
		panic(fmt.Sprintf("xgorm: Dialect %q has no Open", d.Name))
	}

	dialectMu.Lock()
	defer dialectMu.Unlock()
	if _, dup := dialects[d.Name]; dup {
		panic(fmt.Sprintf("xgorm: Dialect %q is already registered", d.Name))
	}
	dialects[d.Name] = d
}

// lookupDialect 取一个已注册的方言
func lookupDialect(name Driver) (Dialect, bool) {
	dialectMu.RLock()
	defer dialectMu.RUnlock()
	d, ok := dialects[name]
	return d, ok
}

// Drivers 返回当前已注册的驱动名，按字母序。
//
// 用来回答「Driver 写对了吗、对应的 module import 了吗」——
// 这是配错驱动时唯一分得清「名字拼错」和「忘了 import」的办法。
func Drivers() []Driver {
	dialectMu.RLock()
	defer dialectMu.RUnlock()
	out := make([]Driver, 0, len(dialects))
	for name := range dialects {
		out = append(out, name)
	}
	slices.Sort(out)
	return out
}

// 内置两个。直接填进 map 而不是在 init 里注册：
// 这样「内置的」和「外部注册的」在源码上一眼可分。
func init() {
	dialects[DriverMySQL] = Dialect{Name: DriverMySQL, Open: openMySQL, Resolve: resolveMySQL, Ready: probeMySQLVersion}
	dialects[DriverPostgres] = Dialect{Name: DriverPostgres, Open: openPostgres, Resolve: resolvePostgres}
}
