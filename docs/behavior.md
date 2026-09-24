# 与底层库不同的默认值与实测

框架接的每个库都有自己的默认行为，其中不少和它的 README、和「大家以为的」不一样。
这份文档记两件事：**框架在哪里改了库的默认值、为什么**，以及**没改的那些实际是什么行为**。
每一条都是用代码量出来的，写着库的默认、这里的默认、实测的数字和当时的依赖版本——
升级依赖之后数字对不上，就说明行为变了，要重新量。

配置项本身见 [`config.md`](config.md)，日志 / 指标 / 链路的字段见 [`observability.md`](observability.md)。

实测环境：没有特别说明的都是本机回环、Go 1.25；带 e2e 的是 `e2e/` 下真起进程、连真服务
（PG 16、MySQL 8.0.46、Redis 7.0.15、ClickHouse 24.8.14）量出来的。

- [总表](#总表)
- [启动期建连探测](#启动期建连探测)
- [TLS](#tls)
- [XGorm：通用](#xgorm通用)
- [XGorm：MySQL](#xgormmysql)
- [XGorm：PostgreSQL](#xgormpostgresql)
- [XGorm：ClickHouse](#xgormclickhouse)
- [XRedis](#xredis)
- [XCache](#xcache)
- [XHttp](#xhttp)
- [XGin](#xgin)
- [XMetric](#xmetric)
- [XTrace](#xtrace)
- [XLog](#xlog)

## 总表

| 配置 | 库自己的默认 | 这里的默认 | 为什么 |
|---|---|---|---|
| `XGin.TrustedProxies` | 全都信（`0.0.0.0/0`） | 一个都不信 | 否则谁发 `X-Forwarded-For` 谁就是访问日志里的 `client_ip`，限流和审计跟着失效 |
| `XGin.MaxMultipartMemory` | 32MB | 8MB | 它是落盘阈值不是请求体上限，堆开销约为它的三倍 |
| `XGin.ReadHeaderTimeout: 0` | 退到 `ReadTimeout`（默认 0），即不限时 | 启动失败 | 发半个请求头就能一直占着连接 |
| `XGin.UseH2C` | x/net 的 `h2c.NewHandler` | 标准库的 `Protocols.SetUnencryptedHTTP2` | 前者劫持连接，`Shutdown` 管不到在途请求 |
| `XGorm.Log: false` | 换成 GORM 自己的 stdout logger | 真的不打 | 那个默认实现带 ANSI 颜色直写 `os.Stdout` |
| `XGorm.Log: true` | 参数值代进 SQL 再记 | 只记带占位符的 SQL | 否则 `WHERE password = ?` 记下来的是真实的密码 |
| `XGorm` 建连 | `gorm.Open` 自己 ping 一次 | 关掉，走框架的 ctx-aware 探测 | 它用自己的 context，退出信号和重试都管不到 |
| `XGorm` MySQL / ClickHouse 查版本 | `Initialize` 里用 `context.Background()` 查 | 挪进建连探测 | 那是第一次建连，失败不重试、不听退出信号 |
| `XGorm` MySQL `parseTime` | `false` | DSN 没写时补 `true` | 否则 `DATETIME` 扫不进 `time.Time` |
| `XGorm` 服务端错误原文 | 原样进日志和 Span | 只记错误码 | 原文带着参数值 |
| go-sql-driver 的日志 | 标准库 log 写 `os.Stderr` | 接到 slog，记成 WARN | 绕开 slog 的纯文本进不了日志平台 |
| `XRedis` 命令超时 | 只认 `ReadTimeout` | 听调用方 ctx 的截止时间 | 不然 200ms 预算的请求会等满 `ReadTimeout` |
| `XRedis` 建连 | 每次建连内部重拨 5 次 | 只拨一次 | 否则主机宕机时一条命令 11.7s |
| `XRedis` 新连接上的握手 | `CLIENT SETINFO`、维护通知都开 | 都关 | 7.2 之前的服务端每条连接留一个报错的 Span |
| `XRedis.Trace` | 整条命令连同参数写进 `db.statement` | 只有命令名 | 否则 `SET` 的值原样进链路后端 |
| go-redis 的日志 | 标准库 log 写 `os.Stderr` | 接到 slog，记成 WARN | 同 go-sql-driver |
| `XCache.MaxCost` | 每条另计 56 字节内部开销 | 只算你给的 cost | 否则 `MaxCost: 2000` 实际只存得下 35 条 |
| `XCache` 停止 | `Close` | `Clear` | `Close` 与并发读写一起跑会 panic |
| `XHttp` 的 resty 日志 | 写 `os.Stderr`，URL 带查询串 | 接到 slog，去掉查询串 | 令牌不落盘 |
| `XHttp` 的 cookie | `resty.New()` 自带 cookie jar | 没有 | 不相干的调用会共享别人种下的会话 cookie |
| `XHttp` 出站 Span | `url.full` 带查询串 | 去掉查询串，Span 名只用方法 | 令牌不进链路后端，Span 名基数有界 |
| `XHttp` 重试条件 | 挂上条件后 resty 自己的判断作废 | 只重试传输层错误 | 否则 200 + 坏 JSON 也会重试 |
| `XTrace` 透传与 baggage | 入站的值照单全收 | 只收直连对端在 `XGin.TrustedProxies` 里的 | 否则公网客户端能伪造 `X-Tenant-Id` 带进内网 |
| `XTrace` 采样 | `AlwaysSample` 无视上游 | `ParentBased`，有上游时听上游 | 否则上游 `sampled=00` 被改成 `-01` 往下传 |
| `XMetric.Namespace` | 不合规的名字导出时转义 | 读配置时失败 | 否则看板按你写的名字查不到 |

下面按模块给出每一项的实测。

## 启动期建连探测

XGorm、XRedis 启动时各探一次，共用同一份实现（`internal/xclient.Probe`，基于 `xutil.Retry`）：

- **最多试 3 次**（第一次加两次重试），两次之间的等待逐次翻倍并带抖动：退避从 1s 起，
  实际等待在 `[0, 当前退避]` 之间取值，两次退避的上界是 1s、2s。
  所以一个实例最多等 `3 × 单次探测预算 + 3s`：XGorm 连 PostgreSQL 默认 `3 × 1.5s + 3s = 7.5s`，
  XRedis 默认 `3 × 1s + 3s = 6s`。
- **认证失败不重试**：PostgreSQL 的 SQLSTATE 第 28 类（密码错、用户不存在都是 28P01）；
  MySQL 的 1045（密码错、用户不存在）和 1044（没有这个库的权限——没有全局权限的账号连一个不存在的库
  拿到的也是 1044；实测 MySQL 8.0.46）；Redis 的 `WRONGPASS` / `NOAUTH`；ClickHouse 的 516 和 192 / 193 / 194。
  错误报 `authentication to <地址> failed`，不是 `cannot reach`。
- **证书被拒不重试**：`x509` 校验失败，或者对端发来 `bad_certificate` / `unknown_ca` /
  `certificate_required` 告警。再试还是同一张证书、同一个结论。
- 启动期间收到退出信号时当场放弃，不等卡着的那次探测撞上超时。

## TLS

实测（e2e：测试时现造的 CA，PG 16、MySQL 8.0.46、Redis 7.0.15 各起一个只收 TLS 的实例）：

| | CA 对：服务端看到的 | CA 不对 / 不填（系统根证书） | `ServerName` 对不上 | 服务端要客户端证书而没带 | 明文连过去 |
|---|---|---|---|---|---|
| XGorm · PostgreSQL | `pg_stat_ssl`：`ssl=t`、`TLSv1.3`，双向认证时 `client_dn=/CN=…` | `x509: certificate signed by unknown authority`，3ms，不重试 | `x509: certificate is valid for localhost, db.e2e.internal, not wrong.e2e.internal` | `FATAL: connection requires a valid client certificate (SQLSTATE 28000)`，按认证失败报 | `no pg_hba.conf entry … no encryption` |
| XGorm · MySQL | `Ssl_version=TLSv1.3`，连接池里每条新连接都是 | 同上，2ms | 同上 | `REQUIRE X509` 的账号回 1045，按认证失败报 | `require_secure_transport=ON` 回 3159，照常重试，1.9s |
| XRedis | 服务端只开 `tls-port` | 同上，1–4ms | 同上 | `remote error: tls: certificate required`，约 0.45s | `EOF`，照常重试，2.0–3.5s |
| XHttp | 桩服务端看到 `HTTP/2.0`、`TLS 1.3` 和客户端证书的 CN | 同上 | 同上 | 报什么说不准，见下 | —— |

两件量出来才知道的事：

- **TLS 1.3 下服务端拒客户端证书，客户端要到下一次读才知道。** 客户端发完 Finished 就当握手成功、开始写，
  服务端的告警晚一步到：go-redis 第一次尝试常常先撞上 `broken pipe` / `EOF`，退避一次之后才读到告警，
  所以是 0.45s 而不是几毫秒；net/http 有时报 `remote error: tls: certificate required`，
  有时报 `write: broken pipe`，HTTP/2 的连接只报 `http2: client conn could not be established`。
- **拿着别的 CA 签的客户端证书，Go 的客户端可能根本不出示它。** 服务端在握手里列出它认的 CA 时，
  crypto/tls 只出示这些 CA 签的证书——PG、net/http 的服务端看到的都是「没带」。
  mysqld 和 redis-server 不列，证书照样发过去，服务端回 `unknown_ca`：go-redis 报
  `remote error: tls: unknown certificate authority`；go-sql-driver v1.10.1 报 `invalid connection`（照常重试），
  告警只出现在一条 `xgorm go-sql-driver log` 里（`detail` 是 `packets.go:58 remote error: tls: unknown certificate authority`）。

**不开 TLS 块时，两个数据库驱动的默认差得很远**：pgx（DSN 不写 `sslmode` 即 `prefer`）连一个开着 ssl 的 PG
照样走 TLS（`pg_stat_ssl` 是 `ssl=t`），但**服务端证书根本不校验**——拿一个不相干的 CA 签的证书也连得上；
服务端不肯 TLS 就悄悄改走明文。go-sql-driver 不写 `tls` 就是明文。
开着 TLS 块时 PostgreSQL 按主机去重、每个主机只走这一块的 TLS，**不会退回明文**（服务端不肯 TLS 时报
`server refused TLS connection`）。

## XGorm：通用

GORM v1.31.2。

**`gorm.Open` 自己 ping 一次**，用它自己的 context：退出信号和重试都管不到。这里关掉（`DisableAutomaticPing`），
改走框架的建连探测。

**不给 Logger 不等于不打日志**：GORM 会补上它自己的默认实现，带 ANSI 颜色直写 `os.Stdout`。
`Log: false` 时这里换成真的什么都不写的 Logger。

**SQL 日志里的参数值。** GORM 的 Logger 不实现 `ParamsFilter` 时把真实参数代进 SQL。
xgorm 的 Logger 实现了它，记的是带占位符的 SQL（MySQL 是 `?`，PostgreSQL 是 `$1`；
GORM 的 PG 方言没参数可代时会写成 `$1$`，这里改回去了）。

**`Scan` 走的是另一条路。** `Raw(...).Scan(...)`、`Table(...).Select(...).Scan(...)` 执行期间 GORM 把实例的 Logger
换成 `logger.Recorder`（`finisher_api.go:539`），它不问 `ParamsFilter`，只认进程级的
`logger.RecorderParamsFilter`，而那个变量的默认值原样返回参数。实测 `WHERE name = ?` 记成了
`WHERE name = 'sql-3fa9c1d2b7e4'`，三种数据库一样。所以 import xgorm 时（它的 `init`）就把这个变量换成
同一个不交出参数的过滤器。它是进程级的：进程里使用者自己 `gorm.Open` 的实例也吃这一条；
要换就在 `main` 里重新赋值。

**服务端的错误原文带着参数值**，SQL 只记占位符也就白记了：

| 服务端 | 错误 | 原文 |
|---|---|---|
| MySQL 8.0.46 | 1062 | `Error 1062 (23000): Duplicate entry 'a@b.com' for key 'm_err.email'` |
| MySQL 8.0.46 | 1366 | `Incorrect integer value: 'notanint' for column 'n' at row 1` |
| MySQL 8.0.46 | 1292 | `Incorrect datetime value: 'secret-date' for column 'd' at row 1` |
| PG 16（pgx v5.10.0） | 22P02 | `ERROR: invalid input syntax for type integer: "notanint" (SQLSTATE 22P02)` |
| PG 16 | 23505 | `Error()` 里没有值，但 `PgError.Detail` 是 `Key (email)=(a@b.com) already exists.` |
| PG 16 | 23514 | `Detail` 是 `Failing row contains (3, chk@y, 500, null, null).` |

所以日志的 `error` 字段和 Span 的状态描述只写 `mysql error 1062 (message omitted, it may contain parameter values)`，
错误码另记（字段见 [`observability.md`](observability.md#数据库)），不调 `RecordError`。**返回给调用方的错误原样不变**，
`errors.As` 照样取得到 `*mysql.MySQLError` / `*pgconn.PgError`。网络错误、ctx 取消、`record not found`
这类客户端一侧的错照原文记。例外是 `database/sql` 的扫描错误，它会引出扫不进去的那个值
（实测 PG：`converting driver.Value type string ("abc-secret") to a int: invalid syntax`），那是列类型与字段类型对不上。

**GORM 自己的默认值**，下面几项不改，量过：

- `SkipDefaultTransaction: false`：每次 `Create` / `Save` / `Update` / `Delete` 都包在一个事务里。
  实测（2000 次 `Create`）PG 每次 3 个往返，跳过是 1 个，572µs → 377µs；MySQL 每次 5 次写（4 个往返），
  跳过是 3 次，1.55ms → 1.33ms。不替你打开，因为它换来的是正确性：`AfterCreate` 钩子返回错误时，
  默认的事务把插入回滚了（0 行），跳过之后那一行留了下来。热点路径上自己用
  `xgorm.C().Session(&gorm.Session{SkipDefaultTransaction: true})`。
- `PrepareStmt: false`：PG 那一侧 pgx 已经按语句缓存了，再开一层是重复；经 PgBouncer 时同样会撞上下面那个问题。
- `NowFunc`：`time.Now().Local()`。PG 上 GORM 给 `time.Time` 建的列是 `timestamptz`，存的是时刻，不受影响；MySQL 见 `parseTime`。
- `TranslateError: false`：打开之后 MySQL 的 1062 变成 `gorm.ErrDuplicatedKey`，但原来的 `*mysql.MySQLError`
  被整个换掉，按错误号判断的代码会静默失效。

**时长不收负数**：底下每一处都会把负数静默变成「不限」——go-sql-driver v1.10.1 的 `FormatDSN` 只写 > 0 的
`timeout` / `readTimeout` / `writeTimeout`；`database/sql` 把负的存活时间当成 0；PG 的 `connect_timeout` 和几个 GUC
在注入时同样被跳过。

## XGorm：MySQL

go-sql-driver v1.10.1、`gorm.io/driver/mysql` v1.6.0、MySQL 8.0.46。

**`parseTime` 默认补成 `true`。** 驱动默认 `false`：`DATETIME` / `TIMESTAMP` 读出来是 `[]byte`，
带 `CreatedAt` 的模型 `First` 一次就报 `unsupported Scan, storing driver.Value type []uint8 into type *time.Time`。
DSN 里写了的（哪怕是 `parseTime=false`）以 DSN 为准；判断照抄驱动的解析规则（最后一个 `/`、第一个 `?` 之后），
密码里的 `?parseTime=false` 不算数。

打开之后时区跟着驱动的 `loc`（默认 `UTC`），写入前转成 `loc`、读出来按 `loc` 解释，同一个时刻来回不变。例如写 `2024-01-02 03:04:05 +01:00`，库里存的是 `2024-01-02 02:04:05`，
读回来是 `02:04:05 UTC`，`Equal` 为 true。库里的墙上时间要给别的系统按本地时间读的，在 DSN 里写 `loc=Local`
（或 `loc=Europe%2FBerlin`）。把 `DATETIME` 扫进 `string` 的，拿到的是 RFC 3339（`2024-01-02T02:04:05Z`），
要原样文本就在 DSN 里写 `parseTime=false`。

**`Initialize` 里的版本查询。** Dialector 初始化时查一次 `SELECT VERSION()`，那行写死了 `context.Background()`，
而且它就是第一次建连，发生在 `gorm.Open` 里、建连重试之前。原样用的话实测（对端收下连接却不回话）：
只试 1 次、3.0s 后失败；启动期间的 SIGTERM 要等握手那一读撞上 `MySQL.ReadTimeout`，信号之后 2.95s 进程才退。
这里关掉（`SkipInitializeWithVersion`），改在建连探测里、每次 Ping 成功之后查。同一场景实测：试满 3 次、
9.6–11.1s 后失败（预算 13.5s）；SIGTERM 之后 7–20ms 退出。版本号照样设进 Dialector，驱动靠它决定的行为
（MariaDB / MySQL 5.x 的改索引名、改列名、`FOR SHARE`、`DROP CONSTRAINT`，MariaDB 10.5+ 的 `RETURNING`）不变。

**单次探测预算**是 `DialTimeout + MySQL.ReadTimeout`（按 DSN 里最终生效的 `timeout` / `readTimeout` 算）。

**取消与截止时间。** 客户端有读超时兜底：对端不回话、调用方又没给截止时间时，查询在 `ReadTimeout`
失败（实测 3.0s，`invalid connection`）。调用方的 ctx 管得更细：

- 给了截止时间就在那一刻返回——池里卡在读上的连接和主机宕机时卡在拨号上的新连接都一样（给 200ms 就是 200ms）；
- ctx 被取消时 go-sql-driver 当场关掉那条连接，查询返回 `context canceled`：客户端放弃之后 0.3ms handler 就返回了。
  被取消的那条连接不还回池里。

新建连接的拨号受 `DialTimeout`（DSN 的 `timeout`）管：主机宕机时 500ms 失败；DSN 里写了 `timeout=1500ms` 的以 DSN 为准。

**`interpolateParams` 默认 `false`**：带参数的查询是 PREPARE、EXECUTE、CLOSE 三次写、两个往返（实测一次 `First` 3 次写），不改。

**go-sql-driver 的日志**默认 `log.New(os.Stderr, "[mysql] ", …)` 写纯文本
（`[mysql] 2026/09/24 10:00:00 packets.go:58 read tcp …: i/o timeout`）。这里在 xgorm 的 `init` 里接到 slog：
一条 `xgorm go-sql-driver log`，级别 WARN，原文在 `detail`。驱动不给 ctx，这些日志不带 trace_id。
实测对端不回话、8 条查询读超时，就是 8 条这样的日志，stderr 里一行 `[mysql]` 都没有。
想换成自己的就在 `main` 里、xone 启动之前调 `mysql.SetLogger`：驱动在解析 DSN 时把当时的 logger 抄进连接配置，
而 xgorm 在启动钩子里才解析 DSN。

## XGorm：PostgreSQL

pgx v5.10.0、`gorm.io/driver/postgres` v1.6.3、PG 16。

**单次探测预算**是「注入的 `connect_timeout`（`DialTimeout` 向上取整到整秒）+ `DialTimeout`」，默认 1.5s。
pgx 拿 `connect_timeout` 管的是每个主机的整个建连（TLS 握手、startup、认证都在里面；pgconn 源码注释原话
"restricts the whole connection process"；实测 startup 不回话的服务端，`connect_timeout=1` 等满 1.0s），
预算比它短的话，一次慢一点但合法的握手会在 pgx 放弃之前被判超时。

**没有读超时。** 连接池里的连接遇上不回话的对端，调用方的 ctx 又没有截止时间，查询就一直等。
`StatementTimeout` 救不了这一段——它是服务端的计时：实测对端吞掉全部字节、`StatementTimeout: 1s`，
一条 `SELECT 1` 30s 后仍没返回。能管住它的只有调用方的截止时间（实测给 200ms 就在 200ms 返回）：

```go
ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
defer cancel()
xgorm.CWithCtx(ctx).First(&u, id)
```

Web 请求里用 `c.Request.Context()` 也行；后台任务、定时任务**一定要自己给截止时间**。

**经 PgBouncer：`default_query_exec_mode`。** pgx 默认 `cache_statement`：每条语句第一次执行时建一个具名预备语句。
PgBouncer 的事务池会把下一条语句派到另一个服务端连接上，具名语句就对不上了。实测 PgBouncer 1.22.0
`pool_mode = transaction`、`default_pool_size = 2`，20 个协程各跑 50 条带参数的查询：

| `default_query_exec_mode` | `max_prepared_statements = 0` | `= 100` | 直连 PG 每条耗时 | 每条写几次 |
|---|---|---|---|---|
| `cache_statement`（pgx 默认） | **701 条失败**：`prepared statement "stmtcache_…" already exists (SQLSTATE 42P05)` | 0 | 59–82µs | 1 |
| `cache_describe` | 0 | 0 | 61–79µs | 1 |
| `describe_exec` | 6 条失败：`unnamed prepared statement does not exist (SQLSTATE 26000)` | 16 条失败 | 68–105µs | 2 |
| `exec` | 0 | 0 | 97–104µs | 1 |
| `simple_protocol` | 0 | 0 | 89–102µs | 1 |

默认值不改：直连 PG 的是大多数。经 PgBouncer 的，二选一：PgBouncer 升到 1.21+ 并把 `max_prepared_statements`
配成非 0（1.24 起默认 200），或者在 DSN 里写 `default_query_exec_mode=exec`（写进 `Postgres.Params` 也生效）。

**DSN 的几个细节**（pgx v5.10.0）：

- key=value 形式里默认值垫在 DSN 前面，pgx 同一个 key 取最后一次，所以 DSN 里写了的自然作数。
- 时区是例外：gorm 的 postgres 驱动另用正则取 DSN 里第一处 `timezone=` / `TimeZone=` / `time_zone=`，
  而且不解码——`TimeZone=Europe/Berlin` 被编码成 `Europe%2FBerlin` 的话每条连接都设不上时区。
  所以 URL 形式里补的参数接在原 query 后面，使用者写的部分原样保留，补进去的值里的 `/` 也不编码。
- 预检用 `pgx.ParseConfig`（比 `pgconn.ParseConfig` 多校验 `default_query_exec_mode` 等三项）。
  pgx 的错误原文是整串 DSN、只遮得住 `password=x` 这种规整写法（`password = hunter2` 原样带出），所以不回传它。
- 多主机 URL 里 IPv6 地址不能排在第一个：`postgres://u:p@[::1]:1,h2:1/db` 会被 `url.Parse` 和 pgx 同时拒绝。
  挪到后面，或者改用 key=value 形式 `host=::1,h2 port=1,1`。

## XGorm：ClickHouse

clickhouse-go v2.48.0（native 协议）、`gorm.io/driver/clickhouse` v0.7.0、ch-go v0.74.0、ClickHouse 24.8.14（e2e）。

**为什么是独立的 module**：只 import xgorm 的应用模块图（`GOWORK=off go list -m all`，不含应用自己的模块）是 64 个，
加上 ClickHouse 驱动变成 127 个（`go list -deps` 里的非标准库包 127 → 171）。多出来的大头是 Docker 和 testcontainers——
clickhouse-go 用它们跑集成测试，而 `go.mod` 分不出「只测试用」。

**`Initialize` 里的版本查询**同 MySQL：写死的 `context.Background()`、发生在建连重试之前。实测对一个收下连接却不回话的
地址，ctx 早已取消也要等满 `dial_timeout`，然后直接失败。这里同样挪进建连探测，版本号照样设进 Dialector
（驱动靠它判断改列名（< 20.4）和列精度（< 21.11））。

**超时与取消：**

- `read_timeout` 没写时是 **300s**（实测对端不回话、不给截止时间：300.6s 后才报错）。
- **`read_timeout` 管的是一整段读，不是「多久没收到字节」。** 一条查询分两段读：读到第一个数据块、再读余下的全部，
  每段开始时设一次 deadline，中途不续期。所以**健康的查询只要余下的结果读得比它久，照样失败**：实测
  50 行 × 100ms 的流配 `read_timeout=1s`，1.0s 报 `i/o timeout`；`SELECT sleep(2)` 同样 1.0s 失败。
  跑得久的查询要么调大 `read_timeout`，要么给调用方的截止时间。
- **调用方的截止时间代替 `read_timeout`**，比它长也照截止时间来（同一条 5s 的流配 `read_timeout=1s`、截止时间 10s，5.0s 读完）；
  池里的连接给 200ms 就在 201ms 返回。**新建连接不听 ctx**：拨号和握手只按 `dial_timeout`，
  池里的连接用完之后每条查询要等满 `dial_timeout`（默认 500ms，实测 500.2–501.5ms）。
- **读超时的查询会被 `database/sql` 重发，一共发 3 次。** 驱动把读超时认成坏连接、报 `driver.ErrBadConn`，
  `database/sql` 换一条连接再发（`maxBadConnRetries` = 2）。实测 `read_timeout=1s`：服务端要睡 1.5s 的查询 3.0s 后才失败，
  `system.query_log` 里它开始了 3 次，返回的错误是 `driver: bad connection`（`i/o timeout` 这个根因丢了）。
  所以给查询的截止时间要比 `read_timeout` 短。
  clickhouse-go v2.30.0 时相反：读超时的连接还回池里，下一条借到它的查询**成功返回了上一条的结果**（上游 v2.47.0 修掉）。
- ctx **取消**时当场返回 `context canceled`，同时给服务端发 Cancel 包、关掉这条连接：客户端断开之后 0.5ms handler 就返回了。
  服务端按数据块停下（约 115ms 从 `system.processes` 消失），`sleep(2.5)` 打断不了。
- `Ping` 只认截止时间、不认取消。建连探测每次都带截止时间（`2 × dial_timeout`），启动期间收到退出信号约 400ms 后退出。
- 参数由驱动代进语句再发给服务端：浮点数写成 `cast(1.5, 'Float64')`，在 `system.query_log` 里按语句文本找时要按这个写法找。
- 驱动对写入报的影响行数永远是 0，Span 的 `db.rows_affected` 在 ClickHouse 上没有意义。

**认证失败**认的是服务端回的 `*clickhouse.Exception`（HTTP 协议下包在 `*clickhouse.HTTPError` 里）：
516（AUTHENTICATION_FAILED）、192 / 193 / 194（老版本的 UNKNOWN_USER、WRONG_PASSWORD、REQUIRED_PASSWORD）。
实测密码错、用户不存在都是 `code: 516`，只试 1 次、50ms 内失败；HTTP 协议下是 `[HTTP 403] code: 516`；
库不存在是 81，不算认证失败，照常试满 3 次。

**TLS**（`verificationMode=relaxed`）：native 与 `https://` 都走 TLS 块，`system.query_log` 都是 `is_secure=1`；
CA 不对、`ServerName` 对不上、客户端证书不是服务端认的 CA 签的都在 2–6ms 内失败、不重试；
TLS 块开着连到明文端口是 `first record does not look like a TLS handshake`（native）/
`server gave HTTP response to HTTPS client`（HTTPS），照常重试。不开 TLS 块时 DSN 的 `skip_verify=true`
连得上，但服务端证书根本不校验。GORM 的 clickhouse 驱动拿到 DSN 会另解一份、在带 `UpdateLocalTable` 的 UPDATE
里按那一份直连每台主机（`update.go`），那几条直连不带 TLS 块——所以开着 TLS 块时不把 DSN 交给它。

多主机 `clickhouse://u:p@h1:9000,h2:9000/db` 按 `in_order` 依次去连，第一个挂了之后查询照常（e2e）。
DSN 解析失败的错误不回显 DSN：驱动和 `url.Parse` 的原始错误里带着整串 DSN，连同明文密码。

## XRedis

go-redis v9.22.0、redisotel、Redis 7.0.15。

**命令听 ctx 的截止时间，不听取消。** go-redis 默认连截止时间都不听，只认 `ReadTimeout`：实测 200ms 预算的请求
在慢 Redis 上等满 5s。这里打开 `ContextTimeoutEnabled`，截止时间设成 socket 的 deadline。但 ctx 被**取消**叫不醒一个
已经阻塞在读上的命令（`ReadTimeout: 1s`、100ms 时取消，命令 1.0s 返回），xredis 在里面补不上：结果写在调用方拿着的
`*Cmd` 上，提前返回就是和还在读的协程抢着写。落到停止流程上：XGin 到点断连时取消了请求的 ctx，卡在 Redis 读上的
handler 要等 `ReadTimeout`，或者等 xredis 的停止钩子关掉连接池（实测 `ReadTimeout: 10s`、`WithStopTimeout(3s)`：
1.6s 断连，handler 到 2.0s 连接池关掉时才返回，`Stop` 报 `1 handler(s) still running`）。要停得下来就给 ctx 带截止时间。

**一条命令最多要多久**（调用方没给截止时间时）：

```
(MaxRetries+1) × (DialTimeout+ReadTimeout) + MaxRetries × MaxRetryBackoff
```

默认 4 × 1s + 3 × 1s = 7s，`MaxRetries: -1` 时是 1s（`MaxRetries` 默认 3、`MinRetryBackoff` 10ms、`MaxRetryBackoff` 1s）。
这条式子成立是因为一次建连只拨一次号：go-redis 默认在每次建连里还藏着一层重试（`DialerRetries` 5 次、间隔 100ms），
一次「建连」就是 5 × 500ms + 4 × 100ms = 2.9s，实测主机宕机（SYN 没有回音）时一条命令用了 11.7s。
xredis 把 `DialerRetries` 固定成 1，同样的场景默认配置 2.1s、`MaxRetries: -1` 时 0.5s。
连接池累计 `PoolSize` 次建连失败之后 go-redis 不再拨号、直接报上一次的错。

**每条新连接上发什么**（挂着 redisotel 数 Span）：

| | go-redis 默认 | 这里 |
|---|---|---|
| `HELLO` | 每条新连接一次，协商 RESP3；服务端不认就退回 RESP2。配成 2 也照样发 `HELLO 2` | 不改 |
| `CLIENT SETINFO` | 每条新连接多一个 pipeline，只为让 `CLIENT LIST` 显示库名。Redis 7.2 之前没有这个子命令：7.0.15 上**每条连接**回 `unknown subcommand 'setinfo'`，错误被吞掉，但每条连接留下一个报错的 `redis.pipeline` Span——`ConnMaxLifetime` 每 5 分钟换一轮连接，就每 5 分钟一批 | 关掉（`DisableIdentity: true`） |
| `CLIENT MAINT_NOTIFICATIONS` | auto：每条新连接先试一次，服务端认就开启「维护通知」，维护期间把读写超时临时放宽（源码默认 10s，手头没有认这条命令的服务端，未实测）。7.0.15 回 `unknown subcommand`，并发建起来的头几条连接各留一个报错的 Span | 关掉：放宽到 10s 和 `ReadTimeout` 的承诺对不上 |

**每条连接的内存**：32KiB 读缓冲 + 32KiB 写缓冲，实测（200 条空闲连接）每条约 66KiB 堆。连接池涨满时是
`PoolSize × 66KiB`：默认 `PoolSize` 是 10 × GOMAXPROCS，4 核 40 条约 2.6MB，64 核 640 条约 41MB；
平时只有 `MinIdleConns`（默认 5）条。缓冲区大小不开放配置。`MaxConcurrentDials` 默认等于 `PoolSize`，不另设。

**TLS 握手受 `DialTimeout` 管**（go-redis 用 `tls.DialWithDialer`，拨号和握手共用一个超时）。

**`Trace`**：redisotel 默认把整条命令连同参数写进 `db.statement`（实测 `SET` 的值原样出现），这里关掉了
（`WithDBStatement(false)`）。钩子不是零成本：没装链路时实测每条命令约 +3µs、+8 次分配。

**go-redis 的日志**默认用标准库 log 往 stderr 写纯文本（`redis: 2026/09/24 10:00:00 pool.go:762: ...`）。
这里在 xredis 的 `init` 里接到 slog：一条 `xredis go-redis log`，级别 WARN，原文在 `detail`。
用命令 ctx 记的那些带 trace_id；最常见的 `failed to dial` 不带——go-redis 在它自己的协程里用
`context.Background()` 拨号，实测 Redis 拒绝连接时 45 条一条都没有 trace_id。自定义的 logger 在 `main` 里、
或在一个 import 了 xredis 的包里设置；放在没有 import xredis 的包的 `init` 里，可能被 xredis 盖掉。

## XCache

ristretto v2.4.2。

**`MaxCost` 默认含内部开销**：ristretto 每条另计 56 字节，`MaxCost: 2000`、cost 为 1 时实际只存得下 35 条。
这里关掉（`IgnoreInternalCost`），统计的只是你给的 cost。按字节记 cost 时把这部分算进自己的预算。

**`DefaultTTL` 为负**：ristretto 会把 ttl < 0 的写入直接丢掉，一条都存不进去，所以读配置时就失败。

**停止时不调 `Close`，只调 `Clear`。** `Close` 先关内部 channel、最后才标记已关闭，和它并发的 `Set` / `Del` / `Get`
会直接 panic（实测 100 轮并发读写里 750 个协程 panic）。使用者拿到的是原生 `*ristretto.Cache`，停止钩子跑的时候
谁还攥着它，框架不知道。`Clear` 对并发读写是安全的；代价是两个后台协程和计数器内存（默认 `NumCounters` 约 4.3MB）
留到进程退出。自己用 `xcache.New` 建、并确定调用方都已停下的，可以直接 `Close()`。

**`Metric` 的开销**：ristretto 的计数默认关着。打开之后每个实例常驻多约 84KB；另有一张记录写入时间的表
（最多 10 万条），写过 10 万个以上不同的键之后多约 8–12MB。本包的 Get / Set 基准前后差异不显著，
ristretto 自己的 4 协程并发 Get 每次约多 20ns（110ns → 131ns）。

**TTL 到期不是到点就移除**：ristretto 按 5s 一个桶、每 2.5s 扫一轮，到期之后几秒才计进 `cache_keys_evicted_total`。
显式 `Del` 和 TTL 到期也算在 evicted 里（同一个计数）。

## XHttp

resty v2.17.2、otelhttp v0.71.0、Go 1.25。

**`Timeout` 管一次尝试，不是一次逻辑请求。** 开了 `RetryCount` 之后最坏是 `(RetryCount+1) × Timeout` 加上几次退避：
`Timeout: 300ms` 配 `RetryCount: 3`，实测跑了 1.24s。要给整次逻辑请求封顶，用调用方的 ctx（`xhttp.R(ctx)`），
每次尝试和中间的退避都听它的。

**重试什么**：和 resty 自己的默认一致，只重试传输层的错（建连失败、超时、连接被重置、响应体没收全），
拿到了响应就不重试——5xx 也不重试。挂上重试条件会让 resty 自己的判断整个作废，所以这条是 xhttp 自己守着的：
不守的话实测 200 + 坏 JSON、`RetryCount: 3` 时同一个 GET 发了 4 次。`RetryOnlyIdempotent` 默认开着：
传输层超时分不出「请求没到服务端」和「处理完了但响应丢了」，重发一个 POST 就可能重复下单。

**连接池没配的那些是标准库的默认**（从 `http.DefaultTransport` 克隆）：

| 项 | 默认 | 实测的行为 |
|---|---|---|
| 代理 | `ProxyFromEnvironment` | 设了 `HTTP_PROXY` / `HTTPS_PROXY` 就全部出站都走代理，`http://` 的请求连同查询串原样交给代理；回环地址不走，`NO_PROXY` 可以排除。环境变量第一次用到时读一次就缓存，之后再改不生效 |
| TLS 握手超时 | 10s | 对端收下 TCP 连接不回握手，10.0s 报 `TLS handshake timeout` |
| 等响应头 | 不限 | 由 `Timeout` 管住整次尝试；`Timeout` 也配 0 的话会永远挂着 |
| `MaxConnsPerHost` | 0，不限 | 对一个 300ms 才回的下游并发 200 个请求，它收到 200 条新连接；紧接着再来 200 个，只有 `MaxIdleConnsPerHost` 那 10 条复用得上 |
| 重定向 | 标准库：最多跟 10 次 | 第 11 次报 `stopped after 10 redirects`。跨 host 跳转时只去掉 `Authorization`、`Cookie` 这几个，**自定义的凭证头（如 `X-Api-Key`）照样带给新 host**；302 把 POST 变成不带 body 的 GET，307 保留方法和 body |
| HTTP/2 | `ForceAttemptHTTP2: true` | 换了 `TLSClientConfig`（TLS 块）之后照旧协商出 `HTTP/2.0` |

不想跟随重定向：`xhttp.C().SetRedirectPolicy(resty.NoRedirectPolicy())`。

**`IdleConnTimeout` 要小于下游的 keep-alive 超时**：对端先关掉空闲连接时，恰好在那一刻复用它的请求会失败。
实测服务端空闲超时 200ms、请求间隔在 200ms 上下：300 个 POST 失败 27 个（`connection reset by peer`），
GET 由标准库自动在新连接上重发，200 个一个没失败。

**跟 resty / otelhttp 默认不一样的地方：**

| | 库自己的默认 | 这里 |
|---|---|---|
| resty 的日志 | 写 `os.Stderr`；开了重试后每次失败打一行 `WARN RESTY Get "http://…?token=…": …, Attempt 1`，用完再打一行 ERROR | 接到 slog（`xhttp resty log`，内容在 `detail`），级别照搬，URL 去掉查询串 |
| cookie jar | `resty.New()` 自带一个，同一 client 的所有请求共享会话 cookie | 没有（用 `NewWithClient`）。初始化前 / 关闭后的兜底实例也没有 |
| 出站 Span 名 | 常见写法是 `GET /users/42`，基数随 id 增长 | 只用方法 `GET`（OTel 语义约定在没有路由模板时的写法） |
| `url.full` | otelhttp 只去掉 `user:password`，查询串原样写进去 | 查询串和片段一并去掉 |
| `CloseIdleConnections` | otelhttp 的 Transport 没实现，`http.Client.CloseIdleConnections()` 断在它那一层，整条调用变成空操作 | 关闭时 xhttp 直接关它自己持有的连接池 |

## XGin

gin v1.12.0、Go 1.25 net/http。

**`TrustedProxies`**：gin 默认 `0.0.0.0/0`，任何人发 `X-Forwarded-For: 1.2.3.4` 就能决定 `client_ip`。
这里默认一个都不信。

**`MaxMultipartMemory`** 不是请求体上限，是「超过多少才落盘」，超出的部分写进临时文件、不会被拒绝。
实际代价约是这个数的三倍：一次 60MB 的上传，配 32MB（gin 默认）时解析这一步让堆多占 96MB，8MB 是 24MB，1MB 是 3MB。

**`ReadHeaderTimeout` / `IdleTimeout` 写 0** 在 net/http 里退到 `ReadTimeout`（默认 0），结果是不限时：
实测发半个请求头的连接一直不被断开。所以两者都必须 > 0。

**`UseH2C`** 用标准库的 `Protocols.SetUnencryptedHTTP2`，不用 x/net 的 `h2c.NewHandler`：后者把连接劫持走，
`Shutdown` 约 60µs 就返回 nil，在途请求照跑。只认先验知识的 h2c（gRPC、`curl --http2-prior-knowledge`），
`Upgrade: h2c` 握手拿到的是普通 HTTP/1.1 响应。

**`MetricPath`** gin 不拒绝不以 `/` 开头的写法，而是悄悄改写：`metrics` 注册成 `/metrics`，访问日志却跳不过它；
留空挂在根路径 `/` 上，业务再注册首页时 gin 在业务自己的路由代码里 panic。所以读配置时就失败。

**`Mode`** 在 `gin.New` 之前设：debug 模式下 `gin.New` 会打一段警告，之后每条路由再各打一行。`gin.SetMode` 是进程级的。

**TLS 与双向认证**（e2e，`MinVersion: "1.3"`）：带着 `ClientCAFile` 的 CA 签的证书是 `200`、`HTTP/2.0`、`TLS 1.3`；
不带证书，客户端报 `remote error: tls: certificate required`；拿别的 CA 签的证书，Go 的客户端根本不出示它，结果同上；
最高只到 TLS 1.2 的客户端报 `protocol version not supported`；明文 HTTP 打到这个端口，net/http 回
`400 Client sent an HTTP request to an HTTPS server.`。这几种一次都没进 handler。
`MinVersion` 默认 1.2，Go 1.25 服务端自己的默认也是 1.2，照样显式写上：默认值会随 Go 版本变。

**优雅退出**：

- `http.Server.Shutdown` 超时只返回错误，在途连接照跑。所以 `Shutdown` 只用到截止时间前的一截（留出剩余时间的 20%，
  最多 1s），到那时还有请求就 `Close()` 断开所有连接。
- 断开连接不等于 handler 返回了：`Close()` 只关连接、取消请求的 ctx。留出来的那一截用来等 handler 真正返回，
  到截止时间还有没返回的，错误里写明几个（`N handler(s) still running when the shutdown deadline passed`）。
- 单独调 `Stop`、传不带截止时间的 ctx 时一直等到在途请求全部做完（实测 1.5s 的请求，`Shutdown` 等了 1.57s）。
- 被劫持走的连接（WebSocket）不归 `Shutdown` / `Close()` 管，它的 handler 同样算在「还没返回」里。

**`http.ErrAbortHandler` 中止的请求**（`httputil.ReverseProxy` 转发到一半上游断开时也是这样）往往已经写出了 200 的响应头，
照读 `c.Writer.Status()` 的话一个被截断的响应记成成功，所以访问日志、指标、链路里记 499，见
[`observability.md`](observability.md#499中止的请求)。

## XMetric

client_golang v1.24.1。

| 写法 | 不拦的话（实测） |
|---|---|
| 桶不是严格递增（`[1, 0.5, 2]`、`[1, 1]`），或含 NaN / Inf | 通过启动，第一次 `HistogramObserve` 时在业务请求里 panic |
| 桶写成空列表 `[]` | 不是「用默认」：被 Prometheus 悄悄换成它自己的 `DefBuckets`（HTTP 那组少了 1ms 一档） |
| `Namespace` 或 `ConstLabels` 的 key 不合规 | 不报错，导出时转义：`my-app` 导出成 `my_app_…`，`1app` 成 `_app_…`，中文成一串下划线 |
| `ConstLabels` 的 key 是 `le` / `quantile` | `le` 通过启动，第一次 `HistogramObserve` 时 panic；`quantile` 让建 Summary 当场 panic |
| `ConstLabels` 的 key 撞上框架自带指标的变量标签 | 撞名的那个注册失败，只打一条错误日志，那组指标一个都导不出去 |
| `ConstLabels` 的 key 是 `version` | `go_info` 自带常量标签 `version`，client_golang 拒绝注册整组 Go 运行时指标（`attempted wrapping with already existing label name "version"`） |

所以这些在读配置时就失败。

## XTrace

OTel SDK v1.46.0、otelhttp v0.71.0。

**采样**：SDK 的 `AlwaysSample` 无视上游的 `sampled=00`，还把 `-01` 往下游传。这里用
`ParentBased(TraceIDRatioBased(SampleRatio))`：有上游时一律听上游的 sampled 位，`SampleRatio: 1` 也不例外。

**`service.name` 的优先级**，后面的压过前面的：OTel 自己的兜底名 `unknown_service:<可执行文件名>` →
`App.Name` / `App.Version` → `OTEL_RESOURCE_ATTRIBUTES` → `OTEL_SERVICE_NAME`。没配的那一项不写，
不会写进一个空的 `service.name=""` 把兜底名盖掉。

**resource 采集出错时只打一条告警、用采到的那部分继续**：`OTEL_RESOURCE_ATTRIBUTES` 写错一项、
或容器里以随机 UID 运行查不到当前用户，都不让服务起不来。

**透传只收可信对端的值。** OTel 自带的 `propagation.Baggage` 谁发来的都收：公网客户端发一个 `X-Tenant-Id`，
或者改写成 `baggage: tenant=…`，就被当成自己人给的、带进内网的每一次调用。规则见
[`observability.md`](observability.md#传播与信任边界)。

**`*trusted.com` 这类通配**：原样照字面后缀匹配的话 `*trusted.com` 会匹配 `eviltrusted.com`，
一个谁都能注册的域名就拿到了内部令牌。所以 `Domains` 只认 `api.internal.com` 和 `*.trusted.com` 两种写法。

## XLog

**`Perm` 为什么是字符串**：实测 yaml.v3 把 `0644` 解析成 420（对的），漏掉前导 0 写成 `644` 却是十进制 644 = 0o1204，
不报错。所以按八进制解析字符串，`0644`、`644`、`0o644` 都认。

**`RotateTime` 的文件名后缀**按周期取粒度：一天及以上是 `app.log.20260918`，一小时及以上是 `app.log.2026091815`，
更短是 `app.log.202609181504`。最细到分钟，所以短于 1m 直接启动失败：0s 实际每分钟一个文件，
30s 两个周期落在同一个文件名上。轮转按本地时区对齐。

**`Timezone` 配了却加载不到直接启动失败**，不会悄悄退回本地时区。scratch / distroless 镜像里没有
`/usr/share/zoneinfo`，要在自己的 `main` 包加一行 `import _ "time/tzdata"`（约 400KB）；框架不替你编，
没用到这项的人不该背这 400KB。
