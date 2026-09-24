# 可观测性：日志、指标、链路

框架自己产出的日志字段、指标和 Span 都在这里，照着建看板、写告警、配日志检索。
所有字段名和消息都是英文，名字就是代码里写的那个。开关在 [`config.md`](config.md) 各模块的一节里。

- [日志](#日志)
  - [访问日志](#访问日志)
  - [框架自己的日志](#框架自己的日志)
- [指标](#指标)
- [链路](#链路)
  - [Span 的名字和属性](#span-的名字和属性)
  - [X-Trace-Id 响应头](#x-trace-id-响应头)
  - [499：中止的请求](#499中止的请求)
  - [传播与信任边界](#传播与信任边界)

## 日志

xlog 把 `slog.Default()` 换成按 `XLog` 配好的 handler，业务和框架都写它。

- **`trace_id` / `span_id`**：装了 xtrace 时，用带 ctx 的方法（`slog.InfoContext(ctx, …)`）写的每一条都自动带上；
  xlog 本身不依赖 OpenTelemetry，这一步由 xtrace 经 `xlog.SetTraceExtractor` 接上。
- **请求级字段**：`xlog.AddKV(ctx, "user_id", id)` 在任意调用层级补一个字段，之后同一请求里的每条日志
  （包括访问日志）都带着它。作用域由 xgin 的 `LogScope` 中间件在每个请求开头开好；自己的非 Web 入口用
  `xlog.CtxWithScope(ctx)` 开。

### 访问日志

每个请求结束时一条，消息 `request completed`，级别 INFO（`XGin.Log` 开着时；`LogSkipPaths` 里的路径和指标端点不记）：

| 字段 | 内容 |
|---|---|
| `method` | 请求方法，原样 |
| `route` | 路由模板，如 `/users/:id`；没匹配上路由时是请求路径 |
| `path` | 请求路径，**不带查询串** |
| `status` | 状态码；中止的请求记 `499`，见[下文](#499中止的请求) |
| `elapsed` | 耗时 |
| `client_ip` | 客户端地址：直连对端，或 `XGin.TrustedProxies` 里的代理转发来的 `X-Forwarded-For` |
| `request_headers` | 请求头，凭证类已脱敏 |
| `request_body` | `LogRequestBody: true` 时，最多前 256KB，逐字段脱敏；multipart 和 `application/octet-stream` 只记一句 `omitted` |
| `response_body` | `LogResponseBody: true` 且是文本类响应时，最多前 4KB，逐字段脱敏 |
| `errors` | handler 里 `c.Error(...)` 登记的错误，没有就不写 |
| `trace_id` / `span_id` | 装了 xtrace 时 |

**脱敏**按敏感词匹配，不是按字段名精确匹配：比较前双方都转小写、去掉 `_ - .` 和空格，字段名里**含**任一敏感词就遮。

| | 规则 |
|---|---|
| 默认敏感词 | `password` `passwd` `secret` `token` `authorization` `apikey` `accesskey` `privatekey` `credential` `cookie` `session` `signature` |
| body（JSON / 表单） | 键含敏感词就遮这个值，任意嵌套层级都算；其余的值原样写回 |
| 其它 body（纯文本、XML…） | 定位不了字段，出现敏感词就整个遮掉 |
| 请求头 | 名字在名单里（`Authorization` `Proxy-Authorization` `Cookie` `Set-Cookie` `X-Api-Key` `X-Auth-Token`，加上 `AddSensitiveHeaders` 追加的）**或者**名字含敏感词 |
| 值是 URL 的请求头 | `Referer`、`X-Original-URL`、`X-Original-URI`、`X-Rewrite-URL`、`X-Forwarded-URI` 去掉 `?` 和 `#` 之后的部分 |

`middleware.AddSensitiveFields(...)` 追加敏感词（body 和请求头一起生效），`middleware.AddSensitiveHeaders(...)`
追加精确的头名。大小写按 Unicode 折叠比较，与 `encoding/json` 匹配字段名一致。词表故意不收 `auth`、`key`、`pwd`：
会误中 `author`、`Idempotency-Key`、随机串。

panic 由 Recover 中间件记一条 `panic while handling request`（ERROR，带 `error`、`stack`、`path`、`method`）并回 500；
客户端提前断开导致的写失败记 `connection broken`，不打栈。

### 框架自己的日志

启停与建连各一条，都不含凭证。常用来检索的几条：

| 消息 | 级别 | 字段 |
|---|---|---|
| `loading config` | INFO | `file` |
| `no config file found, using defaults for everything` | WARN | `searched` |
| `starting` / `stopping` | INFO | `hook`（钩子函数名，如 `xgorm.initXGorm`） |
| `xgin listening` | INFO | `addr`、`tls`、`mtls` |
| `shutdown signal received, closing gracefully; send it again to terminate now` | INFO | `signal` |
| `xgorm connected` | INFO | `name`、`driver`、`addr`、`db`、`tls`、`max_open_conns`、`max_idle_conns` |
| `xredis connected` | INFO | `name`、`addr`、`db`、`tls`、`min_idle_conns` |
| `xcache created` | INFO | `name`、`max_cost`、`default_ttl`、`metric` |
| `xgorm ready` / `xredis ready` / `xcache ready` | INFO | `instances` |
| `SQL` / `slow SQL` / `SQL failed` | INFO / WARN / ERROR | `sql`（带占位符）、`elapsed`、`rows_affected`；失败时 `error`、`error_code`；慢查询时 `threshold`（需 `XGorm.Log: true`） |
| `xgorm go-sql-driver log` / `xredis go-redis log` / `xhttp resty log` | WARN（resty 照搬它的级别） | `detail`：三方库原本写到 stderr 的那一行 |
| `xtrace ignored forward headers from an untrusted peer, …` | WARN | 整个进程只打一次 |

## 指标

框架自带的指标，名字前面都加 `XMetric.Namespace`（有的话）和 `XMetric.ConstLabels`：

| 指标 | 类型 | 标签 | 来源 |
|---|---|---|---|
| `http_requests_total` | counter | `method`、`route`、`status` | xgin，`XGin.Metric` |
| `http_request_duration_seconds` | histogram | `method`、`route`、`status` | xgin，桶是 `XMetric.HTTPDurationBuckets` |
| `http_client_request_duration_seconds` | histogram | `method`、`host`、`status` | xhttp，`XHttp.Metric`；一次逻辑请求记一次（含重试和退避），没拿到响应时 `status` 是 `0` |
| `db_pool_open` / `db_pool_in_use` / `db_pool_idle` / `db_pool_max_open` | gauge | `name`（实例名） | xgorm，`XGorm.Metric`，按实例 |
| `db_pool_wait_total` / `db_pool_wait_duration_seconds_total` / `db_pool_closed_max_idle_total` / `db_pool_closed_max_lifetime_total` | counter | `name` | xgorm |
| `redis_pool_connections` / `redis_pool_connections_idle` | gauge | `name` | xredis，`XRedis.Metric`，按实例 |
| `redis_pool_connections_stale_total` / `redis_pool_hits_total` / `redis_pool_misses_total` / `redis_pool_timeouts_total` | counter | `name` | xredis |
| `cache_hits_total` / `cache_misses_total` / `cache_keys_added_total` / `cache_keys_updated_total` / `cache_keys_evicted_total` / `cache_sets_dropped_total` / `cache_sets_rejected_total` | counter | `name` | xcache，`XCache.Metric`，按实例 |
| `cache_cost` / `cache_max_cost` | gauge | `name` | xcache |
| `log_errors_total` | counter | `level`、`caller` | xmetric，`XMetric.LogErrorMetric`，需要 xlog |
| `go_*` / `process_*` | —— | —— | `XMetric.GoMetrics` / `ProcessMetrics` |

几条要知道的：

- **`method`** 收敛到固定集合：`GET` `HEAD` `POST` `PUT` `PATCH` `DELETE` `CONNECT` `OPTIONS` `TRACE`，其余（包括小写的 `get`）
  一律 `OTHER`——方法是自由 token，照抄的话谁都能把时间序列撑爆。
- **`route`** 是路由模板；没匹配上任何路由时是 `unmatched`，不是真实路径。
- **`host`** 是出站请求 URL 的 `host[:port]`，原样照抄，基数等于你调过的目标数。每个新值乘上 `method` × `status`，
  每个组合 15 条时间序列（默认 12 个桶加 `+Inf`、`_sum`、`_count`）。目标来自用户输入或直连一批 IP 的调用另建一个
  `Metric: false` 的客户端。
- **`cache_keys_evicted_total`** 不只是容量满了被挤掉：显式 `Del` 和 TTL 到期被清理的也算在里面。命中率是
  `hits / (hits + misses)`；调原生的 `C().Clear()` 会把计数清零，Prometheus 当成计数器重置。
- 连接池、缓存的指标在被抓取时才读；`Metric: false` 的实例不出现在 `/metrics` 里。
- 指标注册失败（比如同名指标已被注册成别的类型）**不让启动失败**，只打一条错误日志，那组指标导不出去。

业务打点：`xmetric.CounterInc("orders_total", xmetric.T("status", "ok"))`、`defer xmetric.Timer("handle_order")()`
（耗时指标自动补 `_seconds` 后缀，桶是 `XMetric.HistogramBuckets`）；要完整控制就用 `xmetric.Registry()` 拿原生的
`*prometheus.Registry`。

## 链路

xtrace 装好全局的 TracerProvider 和 Propagator。没 import xtrace 时全局的是 OpenTelemetry 的 noop 实现：
各集成照样调 Span 的接口，但什么都不记。

### Span 的名字和属性

| 来源 | Span 名 | 关键属性 |
|---|---|---|
| xgin（入站） | `GET /users/:id`：方法 + 路由模板；没匹配上是 `GET unmatched` | `http.request.method`（收敛过的）、`http.request.method_original`（原始值和收敛值不同时）、`http.route`、`url.path`、`http.response.status_code`、`gin.errors` |
| xhttp（出站） | 只用方法，如 `GET` | otelhttp 的标准属性；`url.full` **去掉了查询串和片段** |
| xgorm | `gorm.create` / `gorm.query` / `gorm.update` / `gorm.delete` / `gorm.row` / `gorm.raw` | 见下 |
| xredis | redisotel 按命令起名 | 只有命令名，没有 `db.statement` 里的参数 |
| xflow | —— | xflow 不开 Span，步骤里自己用 `otel.Tracer(...)` |

<a id="数据库"></a>xgorm 的属性按 OTel 数据库语义约定 v1.43.0（Tracer 带着这一版的 schema URL）：

| 属性 | 值 |
|---|---|
| `db.system.name` | `postgresql` / `mysql` / `clickhouse`（其余驱动按驱动名） |
| `db.namespace` | 库名 |
| `server.address` / `server.port` | 从 DSN 解出的主机、端口（多主机时是第一个） |
| `db.query.text` | 带占位符的 SQL，不含参数值 |
| `db.operation.name` | 发出去的语句的第一个关键字，原样大小写：`SELECT`、`INSERT`……（软删除发出去的是 `UPDATE`） |
| `db.rows_affected` | 影响行数（ClickHouse 上永远是 0） |
| `db.response.status_code` / `error.type` | 服务端报错时的错误码（MySQL 错误号 / PG 的 SQLSTATE）；其余错误 `error.type` 是 `_OTHER` |

状态只在服务端的 5xx（xgin）或 SQL 出错（xgorm）时标成错误；4xx 不算。服务端报错时状态描述只有错误码，
不调 `RecordError`（它会把带参数值的原文写进 `exception.message`）。

### X-Trace-Id 响应头

xgin 的 `Trace` 开着、**并且 import 了 xtrace** 时，每个响应带 `X-Trace-Id: <32 位 trace id>`，从一次调用直接跳到链路。
没装 xtrace 时 Span 是 noop，没有 trace id 可回带；`XGin.Trace: false` 时也不回带。

### 499：中止的请求

handler 以 `http.ErrAbortHandler` 中止的请求（`httputil.ReverseProxy` 转发到一半上游断开时也是这样），访问日志、
指标、链路里的状态码一律记成 **499**，Span 标为错误，访问日志的 `errors` 里带着 `net/http: abort Handler`。
这个码不会发给客户端——连接直接断了；不这样记的话，一个被截断的响应在三处都记成已经发出去的 200。

### 传播与信任边界

入站接、出站带的有三样东西，信任规则不同：

| | 格式 | 入站收谁的 | 出站 |
|---|---|---|---|
| 链路标识 | W3C `traceparent`、`b3` | 谁发来的都接（只是一个 id） | 注入 `traceparent` 和 `b3` |
| `baggage` | W3C `baggage` | **只收可信对端的** | 带上本进程 ctx 里的 baggage |
| 透传 Header | `XTrace.ForwardHeaders` / `ForwardHeaderRules` 里列的 | **只收可信对端的** | `ForwardHeaders` 发给所有下游，`ForwardHeaderRules` 只发给匹配域名的 |

- **可信对端 = 直连的那一跳在 `XGin.TrustedProxies` 里**（TCP 那一跳，不是从 `X-Forwarded-For` 推出来的 client IP）。
  `TrustedProxies` 默认一个都不信，所以默认 baggage 和透传 Header 一个都不收。「谁是自己人」只在这一处说。
- 不可信的对端带着这些头来时各打一条告警，整个进程只打一次。
- 负载均衡一般原样转发客户端发来的头：把它写进 `TrustedProxies` 之前，先在它那里剥掉这些头，否则等于又信了所有客户端。
- `XGin.Trace: false`、`XHttp.Trace: false` 只关这一跳的 Span，上面三样照常接、照常带。`XTrace.Enable: false` 时
  链路标识和 baggage 不再传播，透传 Header 照常。
- 不经过 xgin、自己调 `Extract` 的（比如从消息队列的消息头里取），carrier 要实现 `TrustedPeer() bool` 并返回 `true`
  才会被当作可信，否则一律不收 baggage 和透传 Header。
- 业务代码里读透传的值：`xtrace.ForwardHeaderFromContext(ctx, "X-Request-Id")`。
