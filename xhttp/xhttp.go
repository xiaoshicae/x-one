package xhttp

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/go-resty/resty/v2"
	"go.opentelemetry.io/contrib/instrumentation/net/http/otelhttp"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/trace"

	"github.com/xiaoshicae/x-one/xconfig"
	"github.com/xiaoshicae/x-one/xerror"
	"github.com/xiaoshicae/x-one/xhook"
	"github.com/xiaoshicae/x-one/xmetric"
	"github.com/xiaoshicae/x-one/xtrace"
)

// fallbackTimeout 初始化之前或关闭之后，兜底 client 的超时
//
// 零值超时是「永不超时」而不是「有个默认值」：对端不响应时请求会一直挂着。
// 最难受的是关闭阶段——某个组件在关闭时发一个这样的请求，
// 整个进程的退出流程就被卡在那里了。
const fallbackTimeout = 30 * time.Second

// New 按配置建一个 HTTP 客户端，不碰本包的全局实例。
//
// 一个例外：cfg.Metric 开着时（默认开着）耗时直方图要注册到 xmetric
// 的全局 Registry —— 指标本来就只有一份，注册到别处就导不出去。
// 不想碰它就把 cfg.Metric 关掉。
//
// 返回的 io.Closer 释放连接池里的空闲连接。
func New(cfg Config) (*resty.Client, io.Closer, error) {
	if err := cfg.validate(); err != nil {
		return nil, nil, xerror.Newf("xhttp", "config", "invalid config: %w", err)
	}

	// 自己抓住连接池那一层，不指望 http.Client.CloseIdleConnections 找得到它。
	//
	// 那个方法是靠类型断言往下找的：链路开着时中间隔着 otelhttp.Transport，
	// 而它没有实现这个方法，断言到那里就断了——整条调用变成空操作，
	// 而链路默认是开着的。自己持有，关的时候直接关它。
	pool := tunedTransport(cfg)
	client := newResty(&http.Client{
		Transport: traced(cfg, pool),
		Timeout:   cfg.Timeout,
	})

	if cfg.RetryCount > 0 {
		client.SetRetryCount(cfg.RetryCount).
			SetRetryWaitTime(cfg.RetryWaitTime).
			SetRetryMaxWaitTime(cfg.RetryMaxWaitTime)
		if cfg.RetryOnlyIdempotent {
			client.AddRetryCondition(retryOnlyIdempotent)
		}
	}

	if cfg.Metric {
		// 用 RegisterAs 的返回值：重复注册时它给的是已有那个实例，
		// 记到新建的那个上会永远导不出去
		hist, err := xmetric.RegisterAs(newDurationHistogram())
		if err != nil {
			return nil, nil, xerror.New("xhttp", "register", err)
		}
		installMetrics(client, hist)
	}

	return client, &clientCloser{pool: pool}, nil
}

// newResty 用给定的 http.Client 建 resty，并把 resty 自己的日志接到 slog 上。
//
// 一律走 NewWithClient 而不是 resty.New()：后者自带一个 cookie jar，
// 同一个 client 发出的所有请求共享它——A 服务种下的会话 cookie
// 会被带给之后每一次毫不相干的调用。配置出来的实例本来就没有 jar，
// 兜底实例曾经是 resty.New()，于是两者行为不一致（实测会串 cookie）。
func newResty(hc *http.Client) *resty.Client {
	return resty.NewWithClient(hc).SetLogger(restyLogger{})
}

// traced 在连接池外面包上链路那几层
//
//	client → xtrace.Transport → otelhttp.Transport → scrubURL → 调好参数的 http.Transport
//
// xtrace.Transport 把目标 host 写进 ctx，按域名透传 Header 的规则才能生效。
// otelhttp 无论链路是否采样都会调用全局 Propagator 注入，所以不需要
// 「链路关了就自己注入」的第二种包装——这一点由本包的测试钉住。
func traced(cfg Config, pool http.RoundTripper) http.RoundTripper {
	if !cfg.Trace {
		return pool
	}
	return &xtrace.Transport{
		Next: otelhttp.NewTransport(scrubURL{next: pool}, otelhttp.WithSpanNameFormatter(spanName)),
	}
}

// scrubURL 把 otelhttp 写在 Span 上的 url.full 换成不带查询串的版本。
//
// otelhttp（v0.71）只去掉 URL 里的 user:password，查询串原样写进 url.full——
// 而查询串里常有令牌和签名，它们会跟着 Span 进链路后端。otelhttp 没有
// 改写属性的选项；它是在开好 Span、调下一层之前 SetAttributes 的，
// 所以在它下面这一层用同一个 key 再写一次就覆盖掉了（SDK 对同名属性取后写的）。
// 路径保留：它不进 Span 名，只是一个属性，排查时要用。
//
// 不记录的 Span（没采样、链路关着）丢掉一切属性，就不必再拼这个 URL。
type scrubURL struct{ next http.RoundTripper }

func (s scrubURL) RoundTrip(r *http.Request) (*http.Response, error) {
	if span := trace.SpanFromContext(r.Context()); span.IsRecording() {
		u := *r.URL
		u.User, u.RawQuery, u.ForceQuery, u.Fragment, u.RawFragment = nil, "", false, "", ""
		span.SetAttributes(attribute.String("url.full", u.String()))
	}
	return s.next.RoundTrip(r)
}

// tunedTransport 从 DefaultTransport 克隆再改，保留 TLS、HTTP/2、代理等默认设置
func tunedTransport(cfg Config) http.RoundTripper {
	t, ok := http.DefaultTransport.(*http.Transport)
	if !ok {
		slog.Warn("xhttp cannot tune the connection pool", "default_transport_type", fmt.Sprintf("%T", http.DefaultTransport))
		return http.DefaultTransport
	}

	t = t.Clone()
	t.MaxIdleConns = cfg.MaxIdleConns
	t.MaxIdleConnsPerHost = cfg.MaxIdleConnsPerHost
	t.IdleConnTimeout = cfg.IdleConnTimeout
	t.DialContext = (&net.Dialer{Timeout: cfg.DialTimeout, KeepAlive: cfg.DialKeepAlive}).DialContext
	return t
}

// spanName 出站 Span 只用方法命名。
//
// 入站那边用的是路由模板（GET /users/:id），出站这边没有模板可用——
// 真实路径 /users/42、/users/43 各是一个 Span 名，基数随用户数增长，
// 链路后端按名字建的索引会被撑爆。OTel 的语义约定在没有模板时也是
// 只用方法。要看具体打到哪，看 url.full 和 server.address 属性。
func spanName(_ string, r *http.Request) string {
	return r.Method
}

// restyLogger 把 resty 自己的日志接到 slog 上。
//
// resty 的默认 logger 在建 client 那一刻抓住 os.Stderr，绕开 slog 直接写。
// 实测（resty v2.17，RetryCount=2，目标端口没人监听）是三行 WARN 一行 ERROR：
//
//	WARN RESTY Get "http://host/x?token=…": dial tcp …: connection refused, Attempt 1
//	…Attempt 2、Attempt 3…
//	ERROR RESTY Get "http://host/x?token=…": dial tcp …: connection refused
//
// RetryCount=0 时这条路径不打日志。另一类是配置调用被忽略时的 ERROR，
// 比如链路开着时 transport 不是 *http.Transport，SetTLSClientConfig 就只打一行错误、
// 什么也不做——那是使用者唯一能看到的信号，所以级别照搬，不往下降。
// 不进 slog 就不进日志平台、格式和其余日志对不上；查询串里的令牌还原样落盘。
// 这里接到 slog，消息里的 URL 去掉查询串和片段。
type restyLogger struct{}

func (restyLogger) Errorf(format string, v ...any) { logResty(slog.LevelError, format, v) }
func (restyLogger) Warnf(format string, v ...any)  { logResty(slog.LevelWarn, format, v) }
func (restyLogger) Debugf(format string, v ...any) { logResty(slog.LevelDebug, format, v) }

func logResty(level slog.Level, format string, v []any) {
	slog.Log(context.Background(), level, "xhttp resty log", "detail", stripQuery(fmt.Sprintf(format, v...)))
}

// urlQuery 文本里 http(s) URL 的查询串和片段
var urlQuery = regexp.MustCompile(`(https?://[^\s"?#]*)[?#][^\s"]*`)

// stripQuery 去掉文本里每个 URL 的查询串和片段
func stripQuery(s string) string { return urlQuery.ReplaceAllString(s, "$1") }

// idempotentMethods 可以安全重试的方法（RFC 9110 的幂等方法）
var idempotentMethods = map[string]struct{}{
	http.MethodGet: {}, http.MethodHead: {}, http.MethodOptions: {},
	http.MethodTrace: {}, http.MethodPut: {}, http.MethodDelete: {},
}

// retryOnlyIdempotent 只让幂等方法重试
//
// resty 默认「传输层出错就重试」，不看方法。但超时分不出
// 「请求没到服务端」和「服务端处理完了但响应丢了」，重发一个 POST
// 就可能变成重复下单。
//
// 挂上任何一个重试条件，resty 自己的判断就整个作废，只听条件的
// （resty v2.17.2 retry.go 的 Backoff）。它自己不重试的那些——请求前的中间件
// 失败、响应已经完整收到之后的解析失败——传进条件时已经剥掉了「不重试」
// 的标记，看上去和传输层错误一样。从前这里对幂等方法一律返回 true，实测
// SetResult 遇上 200 + 坏 JSON、RetryCount=3，同一个请求发了 4 次，
// 不挂条件时 resty 只发 1 次。所以这里先自己认一遍「是不是传输层的错」。
func retryOnlyIdempotent(resp *resty.Response, err error) bool {
	if !isTransportError(err) {
		return false // 拿到响应、或者错在拿到响应之后，都不重试，与 resty 的默认条件一致
	}
	method := ""
	if resp != nil && resp.Request != nil {
		method = strings.ToUpper(resp.Request.Method)
	}
	if _, ok := idempotentMethods[method]; ok {
		return true
	}
	slog.Debug("xhttp skipped retrying a non-idempotent method",
		"method", method, "to_allow_set", ConfigKey+".RetryOnlyIdempotent=false")
	return false
}

// isTransportError 报告 err 是不是 resty 默认会重试的那种传输层错误
//
// 建连、发送、等响应时出的错，http.Client.Do 一律包成 *url.Error，
// 它实现了 net.Error；读响应体时连接被重置、超时也都是 net.Error。
// 响应体读到一半连接断了是 io.ErrUnexpectedEOF，resty 同样重试
// （实测 Content-Length 100 只发 10 字节就断开，不挂条件时共发 4 次）。
// JSON 解析失败、中间件返回的错误都不在其中。
func isTransportError(err error) bool {
	var ne net.Error
	return errors.As(err, &ne) || errors.Is(err, io.ErrUnexpectedEOF)
}

// clientCloser 持有连接池本身，而不是外面那个 http.Client
type clientCloser struct{ pool http.RoundTripper }

func (c *clientCloser) Close() error {
	if p, ok := c.pool.(interface{ CloseIdleConnections() }); ok {
		p.CloseIdleConnections()
	}
	return nil
}

// ---- 全局实例 ----

// fallback 初始化之前、以及关闭之后用的兜底 client，带超时。建一次就不再变。
// 与配置出来的实例一样不带 cookie jar，理由见 newResty
var fallback = newResty(&http.Client{Timeout: fallbackTimeout})

var (
	mu      sync.RWMutex
	current *resty.Client // nil 表示还没起来、或者已经关掉了
)

// C 取 resty client。
//
// 不像 xgorm / xredis 那样取不到就 panic：HTTP 客户端不连任何外部资源，
// 没配也能用，所以这里任何时候都返回一个可用的实例。
// 初始化之前、关闭之后拿到的都是带 30s 超时的兜底实例——关闭阶段仍可能有
// 组件发请求，让它带着超时失败，好过拿到一个已经关掉的客户端。
func C() *resty.Client {
	mu.RLock()
	defer mu.RUnlock()
	if current != nil {
		return current
	}
	return fallback
}

// R 开一个绑定了 ctx 的请求，链路和超时才能传到下游。
//
//	resp, err := xhttp.R(ctx).SetResult(&out).Get(url)
func R(ctx context.Context) *resty.Request { return C().R().SetContext(ctx) }

// RawClient 取底层的原生 *http.Client。
//
// 用于需要自己处理响应体的场景，比如 SSE 这类流式请求——
// resty 会把响应整个读进内存，那对流式接口是不对的。
//
// 初始化之前返回兜底实例内部的那个，而不是 http.DefaultClient：
// 后者的超时是 0，请求可以永久挂住。
//
// 直接从 C() 取而不是另存一份：两份关联状态要同步维护，
// 而 resty 的 GetClient 返回的就是它一直在用的那个，取值一致、生命周期一致。
func RawClient() *http.Client { return C().GetClient() }

// ---- 登记 ----

func init() {
	xhook.BeforeStart(initXHttp, xhook.At(xhook.StageClient))
	xhook.BeforeStop(closeXHttp, xhook.At(xhook.StageClient))
}

// liveCloser 由 initXHttp 填好，closeXHttp 用它收尾
var liveCloser io.Closer

func initXHttp(context.Context) error {
	c := DefaultConfig()
	if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
		return err
	}
	return install(c)
}

// install 按配置建好客户端并装成全局默认实例
func install(c Config) error {
	client, cl, err := New(c)
	if err != nil {
		return err
	}

	mu.Lock()
	current, liveCloser = client, cl
	mu.Unlock()

	slog.Info("xhttp ready", "timeout", c.Timeout, "max_idle_conns_per_host", c.MaxIdleConnsPerHost, "retries", c.RetryCount)
	return nil
}

func closeXHttp(context.Context) error {
	mu.Lock()
	cl := liveCloser
	current, liveCloser = nil, nil // 先摘掉再关，C() 随即退回兜底实例
	mu.Unlock()

	if cl == nil {
		return nil
	}
	return cl.Close()
}
