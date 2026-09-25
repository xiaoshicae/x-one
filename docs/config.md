# 配置参考

一个 YAML 文件，框架统一读，按顶层 key 分发给各组件。这份文档是**参考**：每个配置块的全部字段、
默认值和最要紧的几条规则。为什么这样定、量出来的数字在 [`behavior.md`](behavior.md)；
怎么在代码里用在 [`guide.md`](guide.md)。

**import 了哪个集成，它就生效**，没写的块全用默认值——`XLog`、`XTrace`、`XMetric`、`XHttp`、`XGin`
不配也照常工作。例外是 **`XGorm`、`XRedis`、`XCache`**：它们要连的东西只有你知道，
没配这一块就一个实例都不建，`C()` 会 panic 并说明「没配」。

- [文件位置与优先级](#文件位置与优先级)
- [Profiles —— 按环境分文件](#profiles--按环境分文件)
- [Import —— 引入别的配置文件](#import--引入别的配置文件)
- [合并规则](#合并规则)
- [占位符](#占位符)
- [通用规则](#通用规则) · [TLS 块](#tls-块)
- [编辑器补全](#编辑器补全)
- 各模块：[App](#app--应用身份) · [XLog](#xlog--日志) · [XTrace](#xtrace--链路) · [XMetric](#xmetric--指标) ·
  [XGorm](#xgorm--数据库) · [XRedis](#xredis--redis) · [XCache](#xcache--本地缓存) · [XHttp](#xhttp--出站-http) ·
  [XGin](#xgin--web-服务) · [XGinSwagger](#xginswagger--接口文档) · [XFlow](#xflow--流程编排)

## 文件位置与优先级

从高到低，第一个给出的就是它：

1. 代码里的 `xone.WithConfigPath("…")`
2. 启动参数 `--config=<path>`（也认 `--config <path>`）
3. 环境变量 `XONE_CONFIG`
4. 约定路径，按顺序：`conf/application.yml`、`conf/application.yaml`、`config/application.yml`、
   `config/application.yaml`、`application.yml`、`application.yaml`（相对进程的工作目录）

前三种是点名要的，文件不存在就启动失败（`config file does not exist: <path>`）；约定路径一个都没有时
只打一条告警、全用默认值。

配置在**第一次有人读**的时候才加载——可能早于 `xone.Run`（比如你在 `main` 里就调了 `xconfig.Unmarshal`）。
那时 `WithConfigPath` 还没生效，用的是 2–4 找到的文件；之后 `Run` 再点名另一个文件会直接报错
（`config was already loaded by an earlier read …`）。所以提前读配置的程序用 `--config` 或 `XONE_CONFIG` 指定文件。

框架自己的启动日志（`loading config`、`starting` 等）默认写 `slog.Default()`，xlog 装好之后自动跟着走它；
想换一个 logger 用 `xone.WithLogger(l)`。

## Profiles —— 按环境分文件

写法和 Spring 一样：`application.yml` 放公共的，`application-{profile}.yml` 放这个环境特有的。

```bash
./app --profile=prod           # 或 XONE_PROFILE=prod
./app --profile=prod,eu        # 多个用逗号分隔，靠后的压过靠前的
```

也可以写在 base 文件里（只能写在 base 文件里）：

```yaml
Profiles:
  Active: ${APP_ENV:dev}   # 列表或逗号分隔的字符串都收；占位符在决定读哪些文件之前就展开
```

- 优先级：`--profile` > `XONE_PROFILE` > 文件里的 `Profiles.Active`。
- profile 文件名由 base 文件推出来，目录和扩展名都跟着它：`--config=/etc/app/svc.yaml` 配 `--profile=prod`
  找的是 `/etc/app/svc-prod.yaml`。
- **与 Spring 的一处不同**：点名的 profile 文件不存在时**直接启动失败**，Spring 是静默跳过。

## Import —— 引入别的配置文件

对应 Spring 的 `spring.config.import`。**相对路径按写着这个 `Import` 的文件所在的目录解析**，不是进程的工作目录：
`conf/application.yml` 里写 `shared.yml` 就是 `conf/shared.yml`。

```yaml
Import:                    # 一个就写字符串，多个写列表，靠后的压过靠前的
  - shared.yml             # 即 conf/shared.yml
  - db.yml
  - optional:local.yml     # optional: 前缀，文件不存在就跳过
```

- 引进来的压过引它的那个文件（import 相当于插在它正下方）。
- 引进来的文件同样有 profile 变体：`db.yml` 配 `--profile=prod` 会再找 `db-prod.yml`；片段的变体**不存在不算错**。
- 同一个文件只读一次（菱形引用只算一次，成环在第二次遇到时断开、不报错）；嵌套最多 16 层。
- 被引进来的文件里可以再写 `Import`，但**不能写 `Profiles`**。
- `Import` 的路径里可以写 `${VAR}`，它在读文件之前就展开。

## 合并规则

多个文件的优先级，从低到高：

```
application.yml  <  它 Import 的（含片段自己的 -prod 变体）  <  application-prod.yml  <  prod 那份 Import 的
```

| 类型 | 规则 |
|---|---|
| map | **递归合并**，两边都有的 key 用优先级高的 |
| 列表 | **整体替换**，不逐元素合并——想追加就把完整的列表写全 |
| 标量 | 优先级高的覆盖 |

**没写的东西叠上来什么都不改**：

| 优先级高的文件里写的 | 结果 |
|---|---|
| 整个文件是空的，或者只有注释 | 不贡献任何东西 |
| `XRedis:`、`XRedis: ~`、`Addr:`（null） | 保留低优先级文件里的值，和 `XRedis: {}` 一样 |
| `Addr: ""`、`Headers: []` | 覆盖成空串、空列表——要清空就这样写 |

- **重复的 key 在每个文件里都是错误**，报出文件和两处的行号。
- **锚点和别名**（`&name` / `*name` / `<<: *name`）在同一个文件内随便用，可以跨顶层块；不能跨文件。
  展开后一个文件超过十万个节点直接启动失败。

## 占位符

| 写法 | 含义 |
|---|---|
| `${VAR}` | 必填，未设置则启动失败（`environment variables not set: VAR`）。凭证都该写成这个形式 |
| `${VAR:default}` | 可选，未设置时用 `default` |
| `Port: ${PORT:8080}` | 按展开后的内容判定类型，进得了 int 字段 |
| `Password: "${PW}"` | 加了引号固定按字符串处理，数字形态的密码、版本号靠这一条 |
| `${PORT:}` 或变量是空串 | 等于**这一项没写**，字段保持结构体里的默认值（不是低优先级文件里的值）。真要空串就加引号：`"${PW:}"` |
| 变量的值恰好是 `null` / `~` | 不当成没写，按字符串处理：字符串字段拿到这个字面量，其他类型的字段报类型错误 |

- 占位符在**全部文件合并完之后**才展开（`Import`、`Profiles` 的路径和值例外）：base 里一个必填的 `${SECRET}`
  被 profile 文件整块覆盖掉了，就不再要求它设置。
- 展开发生在解析后的节点上，不是对原始文本替换：值里有冒号、换行也改变不了 YAML 结构。
- 类型不对时报错里不带展开出来的值：报的是 ``cannot unmarshal !!str `${DB_PASSWORD}` (expanded value redacted) into int``。

## 通用规则

| 规则 | 说明 |
|---|---|
| 默认值 | 预填在结构体里，文件没写的字段保持不变。没有 `*bool` 指针，`Enable: false` 就是 false |
| 什么时候读 | 在 `Start` 之前任何时候：第一次读的时候才加载，在 `main` 里、`xone.Run` 之前读到的也是最终值 |
| 字段拼错 | **启动失败**，报错带文件和行号：`application.yml:3: field Bogus not found in type xgin.Config` |
| 没人读的顶层块 | 全部启动钩子跑完时还没人读过的顶层 key **启动失败**（`config keys [...] are not read by anyone`） |
| 值配错 | 各模块的 `Validate` 在读配置时就跑，报错带文件和行号，一个实例都还没连 |
| 时间 | 写 `30s` / `1500ms` / `1h30m`。写裸数字启动失败——写 `30` 的人想要 30 秒，Go 会给他 30 纳秒 |
| 超时写 0 | 各字段的注释写明 0 的含义。`XTrace.ShutdownTimeout`、`XFlow.RollbackTimeout`、`XGin.ReadHeaderTimeout` / `IdleTimeout` 写 0 启动失败 |
| 列表字段 | 文件里写了就整体替换默认值 |
| map 字段 | 文件里写的**合并**进默认值，所以框架的 map 字段一律没有默认值 |
| 建连重试 | XGorm、XRedis 启动时探一次，**最多试 3 次**，退避从 1s 起逐次翻倍、带抖动。认证失败、证书被拒不重试，报 `authentication to <addr> failed`；其余报 `cannot reach <addr>`。数字见 [behavior.md](behavior.md#启动期建连探测) |

### TLS 块

XGorm（PostgreSQL / MySQL / ClickHouse）、XRedis、XHttp 连出去时的 TLS 都写成同一个块，
字段、默认值、校验规则只有一份（`xtls.Config`）：

```yaml
TLS:
  Enable: true                        # 默认 false；下面几项只在开着时生效
  CAFile: /etc/ssl/internal-ca.pem    # 校验服务端证书的 CA（PEM，可以多张）。空 = 系统根证书
  CertFile: /etc/ssl/client.pem       # 客户端证书，服务端要求双向认证时和 KeyFile 成对填
  KeyFile: /etc/ssl/client-key.pem
  ServerName: db.internal             # 比对证书的名字。空 = 连接地址的主机部分
```

- **没开 `Enable` 却写了别的几项，读配置时就失败**（多半是忘了开）；`CertFile` / `KeyFile` 只配一个同样失败。
- 填了 `CAFile` 就**只认**这个文件里的 CA，系统根证书不再参与。
- 最低 TLS 1.2，证书校验一直开着，**没有跳过校验的开关**——自签证书把 CA 填进 `CAFile`。
  要更细的控制（加密套件、自定义校验）就绕开配置、自己造原生 client。
- 证书文件在建实例时读，读不出来报 op 为 `config` 的错，一次都不连。
- 开着 TLS 块时 DSN 里不能再写 TLS 参数（PG 的 `ssl*`、MySQL 的 `tls=`、ClickHouse 的 `secure` / `skip_verify` /
  `tls_server_name`），两处都说了是配置错误。
- 各驱动在不同情况下报什么错、要多久，见 [behavior.md「TLS」](behavior.md#tls)。

服务端（XGin）那一侧的 TLS 是另外几个字段：`CertFile`、`KeyFile`、`ClientCAFile`、`MinVersion`，见 [XGin](#xgin--web-服务)。

## 编辑器补全

仓库根目录的 `config_schema.json` 是这份文档的机器可读版本，挂上之后 YAML 里就有字段补全、拼错标红和悬停说明：

```jsonc
// .vscode/settings.json —— JetBrains 在 Settings → JSON Schema Mappings 里配
{ "yaml.schemas": { "./config_schema.json": ["conf/application*.yml"] } }
```

它由 `go run ./internal/schemagen` 从 Config 结构体生成，字段说明取自结构体上的注释。块里不认识的字段标红；
单实例 / 多实例两种写法都认、混着写标红；数字和布尔字段也收占位符。**顶层**不认识的 key 不标红——
业务自己的配置块也写在顶层，那由启动时的「没人读的顶层块」拦住。

## App —— 应用身份

链路的 `service.name` / `service.version`、接口文档（XGinSwagger）的默认标题和版本都取自这里。

```yaml
App:
  Name: xone.demo.app      # 建议 team.system.app，默认空
  Version: v1.2.0          # 默认空
```

- 指标不读这一块。要在指标上区分应用，用 `XMetric.ConstLabels`（如 `app: xone.demo.app`）。
- 环境变量 `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME` 压过这里，优先级见 [behavior.md「XTrace」](behavior.md#xtrace)。
- 代码里读：`xapp.Name()` / `xapp.Version()`。

## XLog —— 日志

装进标准库的 `slog.Default()`，业务代码直接用 `slog.InfoContext`。

```yaml
XLog:
  Level: info              # debug / info / warn / error，默认 info
  Format: json             # json / text，默认 json
  AddSource: false         # 记代码位置，有开销，默认关
  Timezone: ""             # 时间戳按哪个 IANA 时区渲染，如 Europe/Berlin；空 = 进程本地时区
  Console: true            # 打到标准输出，默认开
  File:
    Enable: false          # 默认关
    Path: /var/log/app     # 目录
    Name: app.log          # 实际文件带时间后缀，另有同名符号链接指向当前文件
    RotateTime: 24h        # 轮转周期，按本地时区对齐，默认一天，至少 1m
    MaxAge: 168h           # 历史保留时长，默认 7 天；0 = 不清理，负数启动失败
    Perm: "0644"           # 按八进制解析的字符串，0644 / 644 / 0o644 都认
```

- `Timezone` 配了却加载不到直接启动失败；scratch / distroless 镜像要 `import _ "time/tzdata"`。代码里用 `xlog.Location()`
  取生效中的时区：`t.In(xlog.Location()).Format(time.RFC3339)`。
- 文件名后缀随 `RotateTime` 的粒度：≥ 24h 是 `app.log.20260918`，≥ 1h 是 `app.log.2026091815`，更短是 `app.log.202609181504`。
- `Name` 那个位置上已经有一个普通文件（不是符号链接）时启动失败，不会把旧日志吞掉。
- 清理只删 `app.log.<时间后缀>` 这种自己命名的文件，启动时一次、之后每次轮转一次；`app.log.bak`、`app.log.1.gz` 不碰。
- 有链路时每条日志自动带 `trace_id` / `span_id`；请求级字段用 `xlog.AddKV(ctx, k, v)`，见 [observability.md](observability.md#日志)。

## XTrace —— 链路

装好 OpenTelemetry 的全局 TracerProvider 和 Propagator，业务代码用原生的 `otel.Tracer("...")`。
框架不内置任何上报 exporter，需要上报的服务自己 `xtrace.AddSpanProcessor(...)`。

```yaml
XTrace:
  Enable: true             # 默认开。关掉后没有 Span，traceparent / baggage 也不再透传，ForwardHeaders 照常
  Console: false           # 把 Span 打到标准输出，本地调试用，默认关
  SampleRatio: 1           # 根 Span 的采样率 [0, 1]；有上游时一律听上游的 sampled 位
  ShutdownTimeout: 5s      # 退出时等导出完成的上限，必须 > 0，同时不超过框架的停止预算
  ForwardHeaders:          # 向所有下游透传的 Header，默认无
    - X-Request-Id
  ForwardHeaderRules:      # 只发给匹配域名的 Header，默认无
    - Domains: ["api.internal.com", "*.trusted.com"]
      Headers: ["X-Internal-Token"]
```

- **透传的 Header 和 `baggage` 只收可信对端发来的**：直连对端在 `XGin.TrustedProxies` 里才收。`TrustedProxies`
  默认一个都不信，于是**默认什么都不透传**。`traceparent` / `b3` 谁发来的都接。细节见
  [observability.md「传播与信任边界」](observability.md#传播与信任边界)。
- `XGin.Trace` / `XHttp.Trace` 只管开不开 Span，关掉之后透传照常。
- `SampleRatio: 0` 是「不采样但照常生成、透传 TraceID」；要连 Span 都不产生用 `Enable: false`。
- `Domains` 只认 `api.internal.com` 和 `*.trusted.com` 两种写法；`*.trusted.com` 匹配任意层级子域、**不匹配裸域**。
  其余带 `*` 的写法、同一个 header 同时出现在 `ForwardHeaders` 和 `ForwardHeaderRules` 里，都在读配置时失败。
- `AddSpanProcessor` 登记的处理器在 `Enable: false` 时收不到 Span，退出时照样被 Shutdown。

## XMetric —— 指标

```yaml
XMetric:
  Namespace: myapp         # 指标名前缀，默认无
  ConstLabels:             # 附加到所有指标上（含 go_* / process_*），默认无
    env: "${ENV:dev}"
  HTTPDurationBuckets: [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]  # 出入站 HTTP 耗时（秒），此为默认
  HistogramBuckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]           # 快捷方法建的直方图，即 prometheus.DefBuckets
  GoMetrics: true          # Go 运行时指标，默认开
  ProcessMetrics: true     # 进程指标，默认开
  LogErrorMetric: true     # Error 级别日志计入 log_errors_total，默认开（需配合 xlog）
```

- 两组桶不写就是默认值，写了就整体替换；空列表 `[]` 不是「用默认」，读配置时失败。桶必须严格递增。
- `Namespace` 和 `ConstLabels` 的 key 只收「字母、数字、下划线，不以数字开头」；`le`、`quantile`、`version`
  和框架指标自己的变量标签（`level`、`caller`、`method`、`status`、`route`、`host`、`name`）不能用作 `ConstLabels` 的 key。
  都在读配置时失败，理由见 [behavior.md「XMetric」](behavior.md#xmetric)。
- 指标端点由 xgin 挂（`XGin.MetricPath`）；不用 xgin 的服务自己挂 `xmetric.Handler()`。框架自带的指标见
  [observability.md「指标」](observability.md#指标)。

## XGorm —— 数据库

`mysql` / `postgres` 内置，ClickHouse 见下面的[其它驱动](#其它驱动)。单实例写法：

```yaml
XGorm:
  Driver: postgres         # mysql / postgres，默认 postgres
  DSN: "${DB_DSN}"         # 必填
  DialTimeout: 500ms       # 建连超时，注入 DSN（MySQL 的 timeout、PG 的 connect_timeout）；0 = 不注入
  MaxOpenConns: 50         # 必须 > 0
  MaxIdleConns: 50         # 0 = 一条空闲连接都不留
  MaxLifetime: 5m          # 0 = 不限
  MaxIdleTime: 5m          # 0 = 不限
  Log: false               # 把 SQL 接到 slog，默认关；只记占位符，不记参数值
  SlowThreshold: 3s        # 超过就记 warn，需 Log 开启；0 = 不记
  IgnoreNotFound: false    # 「没查到记录」是否不当错误
  Trace: true              # 每条 SQL 一个 Span，OTel 数据库语义约定 v1.43.0
  Metric: true             # 连接池指标 db_pool_*，按实例生效
  MySQL:                   # 仅 Driver: mysql 生效
    ReadTimeout: 3s        # 等一次回包的上限，0 = 不注入（驱动不限时）
    WriteTimeout: 5s
  Postgres:                # 仅 Driver: postgres 生效，随建连发给服务端成为会话级参数
    StatementTimeout: 0s   # 服务端计时，默认不限；管不到网络那头不回话
    LockTimeout: 0s
    IdleInTxTimeout: 0s
    Params: {}             # 任意 PG 运行时参数 / pgx 连接参数，同名时以它为准
  TLS:                     # 规则见「TLS 块」；空 ServerName = DSN 里的主机名
    Enable: false
    CAFile: ""
    CertFile: ""
    KeyFile: ""
    ServerName: ""
```

多实例写法：实例写在 `Clients` 下，`xgorm.C()` 取 `default`，`xgorm.C("report")` 取别的。两种写法不能混用。

```yaml
XGorm:
  Clients:
    default: {DSN: "${DB_DSN}"}
    report:  {DSN: "${REPORT_DSN}", Driver: mysql, MaxOpenConns: 5}
```

- **DSN 里写了的参数以 DSN 为准**，配置里的超时只是默认值。时长一律不能为负。
- **MySQL 的 DSN 没写 `parseTime` 时补成 `parseTime=true`**：`DATETIME` 扫得进 `time.Time`，时区跟着驱动的 `loc`（默认 UTC）。
- **PostgreSQL 没有读超时**：连接卡住时只有调用方 ctx 的截止时间管得住，后台任务一定要自己给。经 PgBouncer 的要改
  `default_query_exec_mode`。都见 [behavior.md「XGorm：PostgreSQL」](behavior.md#xgormpostgresql)。
- 日志、Span 里只有带占位符的 SQL；服务端报错时只记错误码，返回给你的错误原样不变。
- 单次建连探测的预算：MySQL 是 `DialTimeout + MySQL.ReadTimeout`，PG 是 `connect_timeout + DialTimeout`（默认 1.5s），
  其余驱动 `2 × DialTimeout`。详细的量值和 GORM 默认行为见 [behavior.md「XGorm：通用」](behavior.md#xgorm通用)。

### 其它驱动

mysql 和 postgres 内置，其余驱动住在自己的 module 里（ClickHouse 驱动多带进 60 多个模块，见 [architecture.md](architecture.md#二每个集成是独立的-go-module)）。
匿名 import 一行就注册好：

```go
import (
	"github.com/xiaoshicae/x-one/xgorm"
	_ "github.com/xiaoshicae/x-one/xgorm/clickhouse"
)
```

```yaml
XGorm:
  Driver: clickhouse
  DSN: "${CH_DSN}"         # clickhouse://user:pass@host:9000/db，也认 tcp:// http:// https://
  DialTimeout: 500ms       # 注入 DSN 的 dial_timeout，DSN 里已写的不覆盖
```

- 拿到的仍是原生 `*gorm.DB`，配置项和多实例写法都一样。驱动名写错或忘了 import 时启动失败，错误里列出已注册的（`xgorm.Drivers()`）。
- DSN 必须是上面四种 scheme 之一的 URL；解析错误一律不回显 DSN（原始错误里带着明文密码）。
- `read_timeout` 没写时是 300s，而且管的是**一整段读**，健康的长查询也会被它打断；读超时的查询会被重发三次。
  给查询的截止时间要比 `read_timeout` 短。见 [behavior.md「XGorm：ClickHouse」](behavior.md#xgormclickhouse)。
- TLS 块对 native 和 `https://` 都生效（`https://` 不必再写 `secure=true`）；DSN 是 `http://` 时开着 TLS 块启动失败。
- 自己写驱动：在 `xgorm.RegisterDialect` 注册的 `Dialect` 里提供 `OpenTLS` 才收 TLS 块，否则配了 TLS 块就启动失败。

## XRedis —— Redis

单实例 / 多实例两种写法，多实例和 XGorm 一样写在 `Clients` 下（`xredis.C("session")`）：

```yaml
XRedis:
  Clients:
    default: {Addr: "127.0.0.1:6379"}
    session: {Addr: "10.0.0.2:6379", DB: 1}
```

单实例的全部字段：

```yaml
XRedis:
  Addr: "127.0.0.1:6379"
  Username: ""
  Password: "${REDIS_PASSWORD}"
  DB: 0
  DialTimeout: 500ms       # 拨号 + TLS 握手
  ReadTimeout: 500ms       # 调用方没给截止时间时，一次读最多等这么久
  WriteTimeout: 500ms
  PoolSize: 0              # 0 = go-redis 的默认 10 × GOMAXPROCS
  MinIdleConns: 5
  MaxIdleConns: 0          # 0 = 不限
  MaxActiveConns: 0        # 0 = 不限
  PoolTimeout: 1s
  ConnMaxIdleTime: 5m
  ConnMaxLifetime: 5m
  MaxRetries: 0            # 0 = go-redis 的默认 3 次，-1 关闭
  MinRetryBackoff: 0s      # 0 = go-redis 的默认 10ms，-1ns 关闭
  MaxRetryBackoff: 0s      # 0 = go-redis 的默认 1s，-1ns 关闭
  Trace: true              # 每条命令一个 Span，只有命令名、不含参数
  Metric: true             # 连接池指标 redis_pool_*，按实例生效
  TLS:                     # 规则见「TLS 块」；空 ServerName = Addr 的主机部分
    Enable: false
    CAFile: ""
    CertFile: ""
    KeyFile: ""
    ServerName: ""
```

- **负数一律不收**，只有 `MaxRetries: -1`、`MinRetryBackoff: -1ns`、`MaxRetryBackoff: -1ns` 三个「关掉」例外。
- **命令听 ctx 的截止时间，不听取消**：ctx 被取消叫不醒一个阻塞在读上的命令，它照样等到 `ReadTimeout`。
  要在停止时停得下来，给 ctx 带截止时间。
- 调用方没给截止时间时一条命令最多 `(MaxRetries+1) × (DialTimeout+ReadTimeout) + MaxRetries × MaxRetryBackoff`，默认 7s。
- 新连接上不发 `CLIENT SETINFO`、不开维护通知；go-redis 自己的日志接到 slog。数字见 [behavior.md「XRedis」](behavior.md#xredis)。

## XCache —— 本地缓存

基于 ristretto。单实例 / 多实例两种写法同 XGorm（`XCache: {Clients: {hot: {...}, cold: {...}}}`）。

```yaml
XCache:
  NumCounters: 1000000     # 频率计数器个数，建议取预期条目数的 10 倍
  MaxCost: 100000          # 总成本上限；包级 Set 的 cost 固定为 1，所以等于条目数
  BufferItems: 64
  DefaultTTL: 5m           # 包级 Set 用的过期时间；0 = 永不过期，负数启动失败
  Metric: true             # 命中率等指标 cache_*，按实例生效
```

- `MaxCost` 只算你给的 cost，不含 ristretto 每条 56 字节的内部开销。
- 停止时只 `Clear` 不 `Close`（`Close` 和并发读写一起跑会 panic）；自己 `xcache.New` 建的可以自己 `Close()`。
- `xcache.DefaultTTL("name")` 名字写错时和 `C("name")` 一样 panic，不返回 0（0 是永不过期）。
- `Metric` 有内存开销（每实例约 84KB，写过 10 万个以上不同的键后多 8–12MB），见 [behavior.md「XCache」](behavior.md#xcache)。

## XHttp —— 出站 HTTP

不配也能用：`xhttp.R(ctx)` 任何时候都有一个可用的客户端（`Run` 之前和之后是按默认值建的兜底实例）。

```yaml
XHttp:
  Timeout: 60s             # 一次尝试的超时，不是整次逻辑请求；0 = 永不超时
  DialTimeout: 30s
  DialKeepAlive: 30s       # 负数 = 不发 keep-alive 探测
  MaxIdleConns: 100
  MaxIdleConnsPerHost: 10  # 标准库默认只有 2
  MaxConnsPerHost: 0       # 每 host 连接数上限，0 = 不限
  IdleConnTimeout: 90s     # 要小于下游的 keep-alive 超时
  RetryCount: 0            # 默认不重试
  RetryWaitTime: 100ms
  RetryMaxWaitTime: 2s
  RetryOnlyIdempotent: true  # 只重试幂等方法
  Trace: true              # 出站 Span；关掉后 traceparent、baggage、透传 Header 照样带给下游
  Metric: true             # 出站耗时指标 http_client_request_duration_seconds
  TLS:                     # 管这个客户端发出的每一个 https 请求，见下
    Enable: false
    CAFile: ""             # 填了就只认它：公网的 https 下游从此校验不过
    CertFile: ""
    KeyFile: ""
    ServerName: ""         # 填了就拿它比对每一个下游的证书
```

- **要给整次请求封顶用调用方的 ctx**：`xhttp.R(ctx).Get(url)`，每次尝试和中间的退避都听它的。
  `Timeout: 300ms` 配 `RetryCount: 3` 实测跑满 1.24s。
- 只重试传输层的错（建连失败、超时、连接被重置），拿到了响应就不重试，5xx 也不重试。
- TLS 块适合「只调一类内部下游」的客户端；要同时调公网的，另用 `xhttp.New` 建一个。`http://` 的请求不受影响。
- 指标的 `host` 标签就是请求 URL 的 `host[:port]`：目标来自用户输入或直连一批 IP 时基数会失控，那种调用另建一个
  `Metric: false` 的客户端。见 [observability.md](observability.md#指标)。
- 代理、重定向、cookie、日志这些标准库 / resty 默认，见 [behavior.md「XHttp」](behavior.md#xhttp)。

## XGin —— Web 服务

```yaml
XGin:
  Host: "0.0.0.0"
  Port: 8080
  Mode: release            # release / debug / test，默认 release；进程级，多个服务要一致
  UseH2C: false            # 非 TLS 下启用 HTTP/2（只认先验知识，不支持 Upgrade: h2c）
  CertFile: ""             # 与 KeyFile 同时配或同时留空；配了就是 https
  KeyFile: ""
  ClientCAFile: ""         # 校验客户端证书的 CA；配了就是双向认证，需同时配证书
  MinVersion: "1.2"        # "1.2" / "1.3"，只在配了证书时生效
  ReadHeaderTimeout: 10s   # 慢连接攻击的主要防线，必须 > 0
  ReadTimeout: 0s          # 默认不限：限制它会打断大文件上传
  WriteTimeout: 0s         # 默认不限：限制它会打断 SSE、长轮询、大文件下载
  IdleTimeout: 60s         # 必须 > 0
  MaxMultipartMemory: 8388608  # 字节，默认 8MB；是落盘阈值，不是请求体上限
  TrustedProxies: []       # 信任哪些代理的 X-Forwarded-For，默认一个都不信；也决定收不收透传 Header
  Log: true                # 访问日志
  LogSkipPaths: []         # 不记访问日志的路径：以 / 结尾的按前缀，其余精确匹配
  LogRequestBody: false    # 请求体进访问日志（逐字段脱敏），默认关
  LogResponseBody: false   # 响应体进访问日志，默认关
  Trace: true              # 每个请求一个服务端 Span、回带 X-Trace-Id
  Metric: true             # 请求指标，并自动挂上 MetricPath
  MetricPath: /metrics     # 必须以 / 开头；Metric 开着时自动加进 LogSkipPaths
  ZHTranslations: false    # validator 的报错翻成中文，用法见 xgin/trans
```

- **停止没有单独的超时**：`Stop` 等在途请求做完，最多到服务那一段停止预算（`xone.WithStopTimeout` 的 2/3，默认 10s），
  到点断开连接、再等 handler 返回；还有没返回的，错误里写明几个。要调就调 `WithStopTimeout`。
- **真在负载均衡后面时**把它那一段网段写进 `TrustedProxies`（如 `["10.0.0.0/8"]`），写错的网段启动失败。
  负载均衡一般原样转发客户端的头，写进来之前先在它那里剥掉 `X-Tenant-Id` 这类头。
- `ClientCAFile` 管整个端口：`/metrics` 同样要客户端证书。handler 里用 `c.Request.TLS.PeerCertificates` 看是谁。
- 框架不替业务定请求体上限，要限就在中间件里 `http.MaxBytesReader`。开 `LogRequestBody` 之前用
  `middleware.AddSensitiveFields(...)` 补上业务自己的敏感字段，脱敏规则见 [observability.md](observability.md#访问日志)。
- XGin 块在装配（`Engine()` 或 `Start`）那一刻才读；`WithRoutes` 回调里改的设置（如 `SetTrustedProxies`）盖过配置，
  但透传 Header 的可信判断只看配置里的 `TrustedProxies`。同一进程里的第二个服务用
  `xgin.New().WithConfig(c)`，`c` 从 `xgin.CurrentConfig()` 改起。实测见 [behavior.md「XGin」](behavior.md#xgin)。

## XGinSwagger —— 接口文档

单独一个 module，UI 资源不进普通服务。在路由里 `xginswagger.Register(e, docs.SwaggerInfo)`。

```yaml
XGinSwagger:
  Host: api.example.com    # 文档里显示的地址
  BasePath: /api/v1
  Title: ""                # 默认取 App.Name
  Description: ""
  Schemes: []              # 默认留空：沿用注解里的 @schemes，写了才覆盖
  URLPrefix: ""            # UI 挂载路径前缀，默认挂在 /swagger/*any；以 / 开头、不以 / 结尾
```

- 没写的字段一律沿用注解里的值，写了才覆盖；标题、版本默认取自 `App`，第一次访问文档时才填。
- `Register` 在 `Start` 之前任何时候调都行，读到的是最终配置。

## XFlow —— 流程编排

```yaml
XFlow:
  Monitor: true            # 关掉之后 Execute 一次监控回调都不走
  RollbackTimeout: 30s     # 回滚全部步骤的总预算，必须 > 0
```

- 回滚不沿用调用方的 ctx，否则请求一超时补偿必然全部失败——而补偿最需要执行的恰恰是那时候。
- 预算对不看 ctx 的 `Rollback` 同样有效：到点就不再等它，那一步记进 `RollbackErrors`，`Execute` 随即返回；
  被放弃的那一步的协程仍在后台跑、仍可能读写 `data`。
