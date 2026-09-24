package xredis

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"time"

	"github.com/redis/go-redis/extra/redisotel/v9"
	"github.com/redis/go-redis/v9"

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

	// fallbackPingTimeout 配置里推算不出预算时的兜底超时
	fallbackPingTimeout = time.Second
)

// pingInterval 两次尝试之间的间隔。是变量而不是常量，只为让测试能调短——
// 连不上的用例要跑满整轮重试，按一秒算一次就是几十秒。
var pingInterval = time.Second

// New 按配置建一个 Redis 实例，不触碰任何全局变量。
//
// 会先 Ping 一次确认连得上：地址写错、密码不对这类问题应该在启动时暴露，
// 而不是等到线上第一次读缓存。
//
// ctx 限定这轮建连验证的生命期：连不上时要走满一轮重试，
// 收到退出信号就该当场放弃，而不是让进程卡在那里。
func New(ctx context.Context, cfg ClientConfig) (*redis.Client, io.Closer, error) {
	if err := cfg.validate(); err != nil {
		return nil, nil, xerror.Newf("xredis", "config", "invalid config: %w", err)
	}

	client := redis.NewClient(&redis.Options{
		Addr:            cfg.Addr,
		Username:        cfg.Username,
		Password:        cfg.Password,
		DB:              cfg.DB,
		DialTimeout:     cfg.DialTimeout,
		ReadTimeout:     cfg.ReadTimeout,
		WriteTimeout:    cfg.WriteTimeout,
		PoolSize:        cfg.PoolSize,
		MinIdleConns:    cfg.MinIdleConns,
		MaxIdleConns:    cfg.MaxIdleConns,
		MaxActiveConns:  cfg.MaxActiveConns,
		PoolTimeout:     cfg.PoolTimeout,
		ConnMaxIdleTime: cfg.ConnMaxIdleTime,
		ConnMaxLifetime: cfg.ConnMaxLifetime,
		MaxRetries:      cfg.MaxRetries,
		MinRetryBackoff: cfg.MinRetryBackoff,
		MaxRetryBackoff: cfg.MaxRetryBackoff,

		// 一次建连只拨一次号，重试交给 MaxRetries。
		//
		// go-redis v9.22.0 默认 DialerRetries=5、两次之间等 DialerRetryTimeout=100ms，
		// 藏在每一次「建连」里面：一次建连最多是 5×DialTimeout+4×100ms，默认配置下 2.9s，
		// 命令级的 4 次尝试再乘上去，实测 Redis 主机宕机（SYN 没有回音）时一条命令用了 11.7s，
		// 拒绝连接时也有 4×400ms 的白等。配成 1 之后一条命令的上限就是文档里那条式子
		// (MaxRetries+1)×(DialTimeout+ReadTimeout)+MaxRetries×MaxRetryBackoff（默认 7s），
		// 同样的场景实测 2.1s（MaxRetries -1 时 0.5s），拒绝连接时 70–150ms（两轮 e2e）。
		// 只拨一次，DialerRetryTimeout 就用不上了（最后一次失败之后不等），所以不设。
		// 不能写 0：0 和负数在 go-redis 里都会被换回 5
		DialerRetries: 1,

		// 让请求 ctx 的截止时间管住 socket 读写。
		//
		// go-redis 默认不这么做：不开的话，每个命令用的是 ReadTimeout /
		// WriteTimeout 这组固定值，调用方给的 deadline 只是摆设——
		// 一个 20ms 超时的请求照样会在一个慢 Redis 上等满几百毫秒，
		// 上游的超时预算和级联保护跟着一起失效。
		//
		// 它管的只是截止时间，不是取消：go-redis 把 ctx.Deadline() 设成 socket 的读写截止时间，
		// 之后不再看 ctx，ctx 被取消叫不醒一个已经阻塞在读上的命令。实测（v9.22.0）
		// ReadTimeout 1s、命令卡住 100ms 时取消 ctx，命令照样在 1s 返回；xgin 停止时断连
		// 取消了请求 ctx，卡在 Redis 上的 handler 要等到 ReadTimeout、或 xredis 的停止钩子
		// 关掉连接池才返回。xredis 在里面补不上这一截：命令的结果写在调用方的 *Cmd 上，
		// 提前返回就是和还在读的那个协程抢着写同一个对象。所以要让命令能被叫停，给 ctx 带上截止时间
		ContextTimeoutEnabled: true,
	})

	// 这之后任何一步失败都要关掉 client，否则漏一个连接池
	ok := false
	defer func() {
		if !ok {
			client.Close()
		}
	}()

	if cfg.Trace {
		// 关掉 db.statement：redisotel v9.22.0 默认开着，把整条命令连同参数
		// 写进 Span（实测 SET session:1 <值> 原样出现），值里的会话、令牌就此
		// 进了链路后端。与 xgorm 只记占位符 SQL、不记参数是同一条原则
		//
		// TODO: redisotel 默认 WithCallerEnabled(true)，每条命令都 runtime.Callers
		// 找调用方，写进 code.function / code.filepath / code.lineno。实测关掉之后
		// 每条命令 14.0µs → 11.6µs（约 -17%，本机回环、noop provider），代价是
		// Span 里不再有这三个属性。这是对外可见的变化，暂保持默认，待定。
		if err := redisotel.InstrumentTracing(client, redisotel.WithDBStatement(false)); err != nil {
			return nil, nil, xerror.Newf("xredis", "new", "install tracing hook: %w", err)
		}
	}

	if err := ping(ctx, client, cfg); err != nil {
		// 原始错误用 %w 带上，调用方要靠它判断根因。它不含凭证：连不上时是
		// 拨号错误（只有地址），认证失败时是服务端回的 WRONGPASS / NOAUTH 文本
		if redis.IsAuthError(err) {
			return nil, nil, xerror.Newf("xredis", "connect", "authentication to %s failed: %w", cfg.Addr, err)
		}
		return nil, nil, xerror.Newf("xredis", "connect", "cannot reach %s: %w", cfg.Addr, err)
	}

	// 日志里只写地址和库号，密码不进日志——所以也就不需要脱敏
	slog.Info("xredis connected", "addr", cfg.Addr, "db", cfg.DB, "min_idle_conns", cfg.MinIdleConns)

	ok = true
	return client, &clientCloser{client: client, addr: cfg.Addr}, nil
}

// ping 建连验证，失败按退避重试；parent 取消时立即放弃。
//
// 认证失败（WRONGPASS / NOAUTH）不重试：密码不对，再试几次也不对，
// 只是让启动多等两次退避（默认 1s + 2s）。做法是叫停这一轮——和退出信号走同一条路，
// xutil.Retry 看到 ctx 取消就不再等、不再试——再把认证错误本身报上去，而不是报「取消」
func ping(parent context.Context, client *redis.Client, cfg ClientConfig) error {
	ctx, stop := context.WithCancelCause(parent)
	defer stop(nil)
	err := xutil.Retry(ctx, pingAttempts, pingTimeout(cfg), pingInterval, func(ctx context.Context) error {
		err := probe(ctx, client)
		if redis.IsAuthError(err) {
			stop(err)
		}
		return err
	})
	if cause := context.Cause(ctx); redis.IsAuthError(cause) {
		return cause
	}
	return err
}

// probe 一次 Ping。ctx 被取消时当场返回，不等这次读撞上 ReadTimeout。
//
// 理由见 New 里 ContextTimeoutEnabled 的注释：go-redis 只认 ctx 的截止时间，
// 取消叫不醒一个阻塞在读上的命令。实测（v9.22.0）对端收下连接不回话时，启动期间的
// 退出信号要等这次读超时才生效：ReadTimeout 500ms 时信号后约 450ms，配 3s 时约 2.95s。
// 所以 Ping 放到协程里跑，这边同时看着 ctx，取消了就不等它。
//
// 丢下的那个协程不会漏：建连失败时 New 会关掉 client，连接池一关，卡着的读当场返回。
// 截止时间到了照样等它——那一刻 go-redis 自己也返回了，它的 i/o timeout 比 ctx 的错误更说明问题
func probe(ctx context.Context, client *redis.Client) error {
	done := make(chan error, 1)
	go func() { done <- client.Ping(ctx).Err() }()
	select {
	case err := <-done:
		return err
	case <-ctx.Done():
		if errors.Is(ctx.Err(), context.Canceled) {
			return ctx.Err()
		}
		return <-done
	}
}

// pingTimeout 单次 Ping 的超时：建连加一个往返
func pingTimeout(cfg ClientConfig) time.Duration {
	d := cfg.DialTimeout + cfg.ReadTimeout
	if d <= 0 {
		return fallbackPingTimeout
	}
	return d
}

type clientCloser struct {
	client *redis.Client
	addr   string
}

func (c *clientCloser) Close() error {
	if err := c.client.Close(); err != nil {
		return xerror.Newf("xredis", "close", "close %s failed: %w", c.addr, err)
	}
	return nil
}

// ---- 全局实例 ----

// C 取一个 Redis 实例，不带参数时取名为 default 的那个。
//
// 取不到直接 panic，理由见 xclient.Registry.Get。
// 不提供 CWithCtx：go-redis 的每个方法本来就收 ctx，再包一层没有意义。
//
// 可选依赖（配了就用、没配就跳过）用 Has 先判断。
func C(name ...string) *redis.Client { return reg.Get(name...).client }

// Has 报告指定实例是否已配置，供可选依赖判断
func Has(name ...string) bool { return reg.Has(name...) }

// Names 返回已配置的实例名
func Names() []string { return reg.Names() }

// ---- 登记 ----

// instance 一个实例连同它自己的 Metric 开关，理由同 xgorm：
// collector 是进程级的一个，不带着这一位就分不出哪个实例配了 Metric: false。
type instance struct {
	client *redis.Client
	metric bool
}

// reg 具名实例注册表。取实例、找不到时的报错、关闭时摘干净，
// 这些语义在 xgorm / xredis / xcache 之间必须一致，所以共用一份实现。
var reg = xclient.NewRegistry[instance]("xredis", ConfigKey)

func init() {
	xhook.BeforeStart(initXRedis, xhook.At(xhook.StageClient))
	xhook.BeforeStop(closeXRedis, xhook.At(xhook.StageClient))

	// go-redis 的日志是进程级的一个，只能在这里接：New 不碰全局，启动钩子里接又会
	// 盖掉使用者在 main 里自己调的 redis.SetLogger。在 init 里接，main 里再调的那次照样生效。
	// 它不读配置、不做 IO，写的是一个包级变量，和登记钩子是同一类事
	redis.SetLogger(slogLogger{})
}

// slogLogger 把 go-redis 自己的日志接到 slog 上。
//
// go-redis v9.22.0 的默认 logger 是标准库 log，往 stderr 写纯文本
// （`redis: 2026/09/24 10:00:00 pool.go:762: redis: connection pool: failed to dial after 1 attempts: ...`），
// 不进 slog 就不是 JSON、不带 trace_id，进不了日志平台。它的日志没有级别，
// 写出来的几乎都是出错时的补充（建连失败、连接关不掉、通知处理失败），
// 调用方同时也拿到了错误，所以一律记成 WARN。ctx 原样传下去，用命令 ctx 记的那些
// （比如过期时间被截到 1ms）由此带上 trace_id；建连失败那句不带——v9.22.0 在它自己的协程里、
// 用 context.Background() 拨号，实测 Redis 拒绝连接时 45 条 failed to dial 一条都没有 trace_id
type slogLogger struct{}

func (slogLogger) Printf(ctx context.Context, format string, v ...any) {
	slog.WarnContext(ctx, "xredis go-redis log", "detail", fmt.Sprintf(format, v...))
}

// initXRedis 读配置，按名字把实例挨个建出来。
//
// 没配这一块就一个都不建：xredis 是可选依赖，没配不该让服务起不来。
// 但 Build 照样走一遍，注册表由此知道启动钩子跑过了——之后 C() 取不到时
// 报的是「没配」，而不是「调早了」。
func initXRedis(ctx context.Context) error {
	if !xconfig.Has(ConfigKey) {
		return xclient.Build(ctx, reg, nil, build)
	}
	c := DefaultConfig()
	if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
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
	slog.Info("xredis ready", "instances", reg.Names())
	return nil
}

// closeXRedis 摘掉全部实例并逆序关闭。
//
// 启动钩子失败时它不会被调到——停止钩子只在和它配对的启动钩子成功之后才执行，
// 所以这里不必处理「还没建起来」。
func closeXRedis(context.Context) error { return reg.Close() }

// build 建一个实例。包一层 New 而不是直接把 New 交出去，
// 是为了把这个实例的 Metric 开关一起带进注册表
func build(ctx context.Context, c ClientConfig) (instance, io.Closer, error) {
	client, closer, err := New(ctx, c)
	if err != nil {
		return instance{}, nil, err
	}
	return instance{client: client, metric: c.Metric}, closer, nil
}

// installPoolMetrics 把连接池 collector 挂到当前的 Registry 上。
//
// 每次 install 都挂、不用 sync.Once，理由同 xgorm：xmetric 重装之后是新的
// Registry，Once 会把这第二次挡掉；同一个 Registry 上重复挂由 xmetric.Register
// 复用已有的那个。
//
// 不让启动失败：指标导不出去是可观测性问题，不该拦住服务起来。
func installPoolMetrics() {
	if _, err := xmetric.RegisterAs(newPoolCollector(xmetric.Namespace(), xmetric.ConstLabels(), poolStats)); err != nil {
		slog.Error("xredis failed to register pool metrics", "error", err)
	}
}

// poolStats 读开了 Metric 的各实例连接池的实时状态
func poolStats() map[string]*redis.PoolStats {
	out := map[string]*redis.PoolStats{}
	for _, name := range reg.Names() {
		if inst, ok := reg.Lookup(name); ok && inst.metric {
			out[name] = inst.client.PoolStats()
		}
	}
	return out
}
