# 排错

按错误原文查。框架的错误都是 `xone <模块> <op> failed, err=[…]` 的形状（见 [guide.md「错误处理」](guide.md#错误处理)），
下面只列方括号里那一段的关键字，搜日志时用它就行。

- [配置](#配置)
- [取实例：C()](#取实例c)
- [Runnable 与退出](#runnable-与退出)
- [建连](#建连)
- [TLS](#tls)

## 配置

### `config keys [MyApp] are not read by anyone`

全部启动钩子跑完时，配置文件里还有顶层 key 没人读过。

- **拼错了**：`XRedsi`、`Xgin`。对照 [config.md](config.md) 的块名。
- **忘了 import 对应的包**：写了 `XGorm` 块却没有 `import _ "github.com/xiaoshicae/x-one/xgorm"`（或者用它的包）。
- **业务自己的块读得太晚**：只在 `Start` 里才第一次 `xconfig.Unmarshal` 的块不算。挪到 `main` 里、`xone.Run` 之前，
  或者一个 `BeforeStart` 钩子里，见 [guide.md「读自己的配置」](guide.md#读自己的配置)。
- 「配了才用、没配就跳过」的包用 `xconfig.Has(key)` 判断：问过就算认领了。

### `field Bogus not found in type xgin.Config`

前面带着 `application.yml:3:` 这样的文件和行号。字段拼错了，或者没有这个字段——配置里多写的字段一律启动失败。

后面跟着 `(field Endpoint has no yaml tag, so only "endpoint" is accepted: add `yaml:"Endpoint"`)` 的，是你自己的结构体
字段没写 `yaml` tag：yaml.v3 对没写 tag 的字段只认全小写的 key。照提示加上 tag。

### `environment variables not set: DB_PASSWORD`

配置里写了 `${DB_PASSWORD}`（必填），进程的环境里没有它。设上，或者改成带默认值的 `${DB_PASSWORD:…}`。
`environment variables not set in Import of …` / `in Profiles of …` 同理，出在 `Import` 路径或 `Profiles.Active` 里。

变量设成了空串等于「这一项没写」，字段会保持默认值；要空串写 `"${VAR:}"`。

### `config file does not exist: /etc/app/application.yml`

`WithConfigPath`、`--config` 或 `XONE_CONFIG` 点名的文件不在。检查路径；相对路径相对的是进程的工作目录。

### `read config conf/application-prod.yml: … no such file or directory`

激活了 `prod` profile，但 base 文件旁边没有对应的 `-prod` 文件。profile 名多半写错了；框架不会静默跳过它。

Import 进来的片段找不到时同样是这个错误：片段的路径相对的是**写着 `Import` 的那个文件所在的目录**，
不是工作目录。要允许不存在就加 `optional:` 前缀。

### `config was already loaded by an earlier read (from conf/application.yml), so … cannot take effect`

`main` 里在 `xone.Run` 之前就读过配置（`xconfig.Unmarshal`、`xgin.CurrentConfig()`……），那时按 `--config` /
`XONE_CONFIG` / 约定路径已经加载了一份，`WithConfigPath` 再点名另一个文件就对不上了。改用 `--config` 或 `XONE_CONFIG`。

### `duplicate key "Port" at line 5 (first seen at line 3)`

同一个文件里同一层写了两遍同一个 key。删掉一个。

### `cannot mix the single- and multi-instance forms`

`XGorm` / `XRedis` / `XCache` 里既写了 `Clients`，又在块的顶层写了实例字段（`DSN`、`Addr`……）。
要么全挪进 `Clients.<名字>`，要么去掉 `Clients` 用单实例写法。`Clients is empty` 是写了 `Clients` 却一个实例都没有。

### `invalid config XGorm: Clients.report: application.yml:12: MySQL.ReadTimeout must not be negative`

各模块的 `Validate` 在读配置时就跑。报错里点名了实例、文件、行号和字段，照着改那一处。常见的：时长写成负数、
`MaxOpenConns: 0`、`XGin.ReadHeaderTimeout: 0`、`XTrace.ShutdownTimeout: 0`、`XMetric.Namespace` 里有 `-`。

### `unknown Driver="clickhouse", registered: [mysql postgres]`

`Driver` 写错了，或者没 import 驱动的 module（`_ "github.com/xiaoshicae/x-one/xgorm/clickhouse"`）。

## 取实例：C()

`xgorm.C()` / `xredis.C()` / `xcache.C()` 取不到实例时直接 panic，四种情况文案不同，要查的地方也不同：

| panic 里说的 | 原因 | 怎么改 |
|---|---|---|
| `instance "default" was requested before xone.Run started xgorm` | 调早了：在 `main` 里、`init()` 里、包级变量初始化时，或者比 `StageClient` 更早的钩子里 | 挪到 `Start`、请求处理，或默认档（`StageBusiness`）及之后的钩子里；要在包级变量里持有就存函数 `xgorm.C` 而不是它的结果 |
| `instance "default" was requested after xgorm was closed` | 调晚了：退出流程已经把它关了，还有协程在用 | 让 `Start` 等在途工作做完再返回，见 [guide.md「非 Web 服务」](guide.md#非-web-服务consumer--job) |
| `no instance named "default", and none is configured at all — check the XGorm block` | 配置里整块没写 | 写上 `XGorm` 块；可选依赖用 `xgorm.Has()` 先判断 |
| `no instance named "orders", configured ones are [default report]` | 名字写错，或者多实例里没有 `default` 却调了不带参数的 `C()` | 按列出的名字改，或者在 `Clients` 下加一个 `default` |

`xhttp.C()` / `xhttp.R(ctx)` 不会 panic：任何时候都有一个按默认值建的客户端。

## Runnable 与退出

### `main.App has a Stop method of type func(main.App) error, but Run only calls Stop(context.Context) error`

`Stop` 的签名不对（常见是少了 `ctx`）。框架只认 `Stop(context.Context) error`；写错了的话它永远不会被调到、服务收到信号也停不下来，
所以 `Run` 直接报错。改签名，或者这个方法本来就不是用来关闭的就换个名字。

### `main.App has Stop on its pointer receiver, so Run cannot call it on a value: pass a pointer (&main.App{...})`

`Stop` 写在指针上，传给 `Run` 的却是值。传 `&App{...}`。

### `stop budget must be > 0 (0 is not unlimited, it is no wait at all), got=0s`

`xone.WithStopTimeout` 给了 0 或负数。0 不是「不限时」，是「一点都不等」。给一个正数，并让部署环境的终止宽限期比它长。

### `N handler(s) still running when the shutdown deadline passed`

xgin 到了服务那一段停止预算（`WithStopTimeout` 的 2/3，默认 10s）还有 handler 没返回。已经断开了连接、取消了请求的 ctx，
剩下的是**不看 ctx 的 handler**：慢操作要传 `c.Request.Context()`。卡在 Redis 读上的也会这样——go-redis 不认取消，
给 ctx 带上截止时间（`context.WithTimeout`）。真的需要更长就调大 `WithStopTimeout`。

### `server did not exit within its share of the stop budget, closing the rest`

`Start` 没在服务那一段预算里返回（`Stop` 不看 ctx、或者 `Start` 没在 ctx 取消后返回）。框架不再等它，接着关各组件——
这时它要是还在用数据库，就会撞上已经关掉的连接池。让 `Start` 在 ctx 取消后返回。

### 进程卡住不退

第一个 SIGINT / SIGTERM 之后框架开始优雅退出，并把系统默认处置还回去：**再发一次信号进程立即终止**（Ctrl+C 两次）。
日志里 `shutdown signal received, closing gracefully; send it again to terminate now` 之后没有下文，多半是某个停止钩子
或 `Stop` 卡在一个不看 ctx 的调用里。框架不会无限等：总预算 `WithStopTimeout` 到了 `Run` 一定返回。
K8s 里要让 `terminationGracePeriodSeconds` 比 `WithStopTimeout` 长，见 [guide.md「部署」](guide.md#部署)。

### `shutdown signal received during startup, not starting the server`

启动期间（连库、重试时）收到了退出信号。不是故障：已建好的逆序关掉，以 0 退出。

## 建连

### `authentication to db:5432 failed` 与 `cannot reach db:5432`

启动时的建连探测最多试 3 次（[behavior.md](behavior.md#启动期建连探测)）。

- `authentication to <addr> failed`：服务端明确拒绝了凭证（密码错、用户不存在、MySQL 没有这个库的权限、
  PG 要求客户端证书而没带），**只试一次**。查账号密码和权限；后面跟着服务端的原文（不含凭证）。
- `cannot reach <addr>`：连不上、超时、库不存在（MySQL 以外），试满 3 次之后报。查地址、网络、防火墙、
  `DialTimeout`，以及服务端是不是只收 TLS（明文连到 TLS 端口常见的是 `EOF`、`unexpected packet`）。

## TLS

| 错误原文 | 原因 | 怎么改 |
|---|---|---|
| `TLS fields are set but TLS.Enable is false` | 写了 `CAFile` 等却没开 `Enable` | 加 `Enable: true`，或者删掉那几项 |
| `TLS.CertFile and TLS.KeyFile must be set together` | 客户端证书只配了一半 | 两个都填 |
| `the DSN sets sslmode while the TLS block is enabled; configure TLS in one place only` | PG 的 DSN / `Postgres.Params` 里有 `ssl*` 参数，同时开了 TLS 块 | 二选一 |
| `the DSN sets tls while the TLS block is enabled` | MySQL 的 DSN 里写了 `tls=`（哪怕 `tls=false`） | 二选一 |
| `the DSN sets secure while the TLS block is enabled` | ClickHouse 的 DSN 里有 `secure` / `skip_verify` / `tls_server_name` | 二选一 |
| `the DSN uses http:// while the TLS block is enabled, and http:// never runs TLS` | ClickHouse 的 DSN 是 `http://` | 改成 `https://` |
| `Driver="…" does not support the TLS block, configure TLS in its DSN instead` | 这个驱动没提供 `OpenTLS` | 在它的 DSN 里配 TLS |
| `read TLS.CAFile: …` / `TLS.CAFile … contains no PEM certificate` | 证书文件读不出来或不是 PEM | 检查路径和文件内容 |
| `x509: certificate signed by unknown authority` | 服务端证书不是 `CAFile` 里的 CA 签的（没填 `CAFile` 时是系统根证书） | 把签发它的 CA 填进 `CAFile` |
| `x509: certificate is valid for …, not …` | 证书上的名字和连接地址对不上 | 按 IP 连时填 `ServerName` |
| `remote error: tls: certificate required` / `unknown certificate authority` | 服务端要客户端证书，没带或不是它认的 CA 签的 | 填 `CertFile` / `KeyFile`，用服务端认的 CA 签 |
| `CertFile and KeyFile must both be set or both be empty`（XGin） | 服务端证书只配了一半 | 两个都填，或者都留空 |
| `ClientCAFile requires CertFile and KeyFile, mutual TLS runs on top of TLS`（XGin） | 配了双向认证却没配服务端证书 | 补上 `CertFile` / `KeyFile` |

证书被拒不重试：再试还是同一张证书、同一个结论。
