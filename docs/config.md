# 配置参考

一个文件，框架统一读，按顶层 key 分发给各组件。没配的块就是没启用——
除了 `XHttp`，它不连任何外部资源，不配也会给你一个可用的客户端。

配置文件位置：`--config=<path>` > `XONE_CONFIG` > `conf/application.yml` 等约定路径。

## Profiles —— 按环境分文件

写法和 Spring 一样：`application.yml` 放公共的，`application-{profile}.yml` 放这个环境特有的。

```bash
./app --profile=prod           # 或 XONE_PROFILE=prod
./app --profile=prod,cn        # 多个用逗号分隔，靠后的压过靠前的
```

也可以写在配置文件里（只能写在 base 文件里，见下）：

```yaml
Profiles:
  Active: [prod]        # 或 Active: "prod,cn"，逗号分隔的字符串也收
```

```yaml
Profiles:
  Active: ${APP_ENV:dev}  # 占位符可用：这一块在决定读哪些文件之前就展开
```

优先级：`--profile` > `XONE_PROFILE` > 文件里的 `Profiles.Active`。

profile 文件名由 base 文件推出来，目录和扩展名都跟着它：
`--config=/etc/app/svc.yaml` 配 `--profile=prod` 找的是 `/etc/app/svc-prod.yaml`。

> **与 Spring 的一处不同**：点名的 profile 文件不存在时，这里**直接启动失败**，
> Spring 是静默跳过。profile 名写错几乎总是笔误，静默跳过的结果是一份
> 谁都没看过的配置悄悄以默认值起来。

## Import —— 引入别的配置文件

对应 Spring 的 `spring.config.import`。

```yaml
Import: conf/shared.yml           # 一个
```

```yaml
Import:                           # 多个，靠后的压过靠前的
  - conf/shared.yml
  - conf/db.yml
  - optional:conf/local.yml       # optional: 前缀，文件不存在就跳过
```

- **引进来的压过引它的那个文件**，和 Spring 一致（import 相当于插在声明它的那份文档正下方，下面的压过上面的）
- **引进来的文件同样有 profile 变体**：`Import: parts/db.yml` 配 `--profile=prod`
  会再找 `parts/db-prod.yml`。配置拆成片段之后，最该按环境变的恰恰是片段里的
  连接串之类，只有主文件有变体的话拆分会很别扭
- 片段的变体**不存在不算错**（与主文件不同）：拆成十个片段之后要求每个都备齐
  每个环境的变体没法用，而 profile 名写错已经由主文件的变体挡住了
- 相对路径按**引它的文件所在目录**解析，不是进程的工作目录
- 同一个文件只会被读一次（与 Spring 一致），所以两个片段引同一份公共配置时它只算一次，
  循环引用也会在第二次遇到时自己断掉、**不报错**
- 嵌套最多 16 层，再深直接启动失败。成环走不到这条线，它挡的只是一条真的很长的链
- 被引进来的文件里可以再写 `Import`，但**不能写 `Profiles`**：那会让「加载哪些文件」变成一个和加载顺序互相依赖的问题，直接报错

## 合并规则

多个文件的优先级，从低到高：

```
application.yml  <  它 Import 的（含片段自己的 -prod 变体）  <  application-prod.yml  <  prod 那份 Import 的
```

展开成每个文件都一样的形状：**F、F 的 import、F-p1、F-p1 的 import、F-p2……**

叠加规则和 Spring 一致：

| 类型 | 规则 |
|---|---|
| map | **递归合并**，两边都有的 key 用优先级高的那个 |
| 列表 | **整体替换**，不逐元素合并 |
| 标量 | 优先级高的覆盖 |

列表整体替换是最容易误解的一条：逐元素合并的话，base 写 `[A, B]`、
prod 写 `[C]`，结果会是 `[C, B]` —— 你以为换掉了整张表，实际只换掉第一项，
剩下那项来自另一个文件。想追加就把完整的列表写全。

**没写的东西叠上来什么都不改**：

| 优先级高的文件里写的 | 结果 |
|---|---|
| 整个文件是空的，或者只有注释 | 这个文件不贡献任何东西 |
| `XRedis:`、`XRedis: ~`、`Addr:`（null） | 保留低优先级文件里的值，和 `XRedis: {}` 一样 |
| `Addr: ""`、`Headers: []` | 覆盖成空串、空列表——要清空就这样写 |

null 在单个文件里的意思是「保持默认值」，叠加时的意思也就是「保持低优先级的值」。
于是 `application-prod.yml` 里只写一行 `# TODO` 不会让全部配置悄悄退回默认值，
`XRedis:` 和 `XRedis: {}` 也不会是两种结果。

**重复的 key 在每个文件里都是错误**，报出文件和两处的行号，不论顶层还是嵌套、
不论是 base、profile 还是 import 进来的文件。合并按名字对齐 key，
重复在那一步就被吞掉了，不当场报错的话同一个字段写两遍会静默以后一份为准。

**锚点和别名**（`&name` / `*name` / `<<: *name`）在同一个文件内随便用，可以跨顶层块：

```yaml
XGorm:
  DialTimeout: &dial 500ms
  DSN: "${DB_DSN}"
XRedis:
  DialTimeout: *dial         # 引的是 XGorm 里的锚点
  Addr: "127.0.0.1:6379"
```

别名在读文件时就展开成副本，所以 profile 文件能覆盖别名带进来的字段，
锚点里的 `${VAR}` 在每个引用处各自展开。锚点不能跨文件（YAML 本身就不支持）。
展开后一个文件超过十万个节点直接启动失败——正常配置只有几百个，这条线挡的是
别名层层相乘的「十亿笑」。

`${VAR}` 占位符在**全部合并完之后**才统一展开（只有 `Import` 的路径是例外，
那个得先展开才知道去读哪个文件）。所以 base 里一个必填的 `${SECRET}`
如果已经被 profile 文件整个覆盖掉了，就不会再要求它必须设置。

## 编辑器补全

仓库根目录的 `config_schema.json` 是这份文档的机器可读版本，挂上之后
YAML 里就有字段补全、拼错标红和悬停说明：

```jsonc
// .vscode/settings.json —— JetBrains 在 Settings → JSON Schema Mappings 里配
{ "yaml.schemas": { "./config_schema.json": ["conf/application*.yml"] } }
```

它由 `go run ./internal/schemagen` 从 Config 结构体生成，`check.sh` 保证它不会过期，
也保证这份文档每一节的 YAML 示例都过得了它。它和运行时的规则对齐：

- 块里不认识的字段标红（和启动时的「字段拼错直接失败」一致）
- `XGorm` / `XRedis` / `XCache` 的单实例和多实例两种写法都认，混着写标红
- `Import`、`Profiles.Active` 写一个字符串或写列表都行
- 数字和布尔字段也收占位符：`Port: ${PORT:8080}` 不标红
- **顶层**不认识的 key 不标红：业务自己的配置块也写在顶层，schema 不可能认识它们。
  顶层拼错由启动时的「没人认领的块直接失败」拦住

## 通用规则

| 规则 | 说明 |
|---|---|
| 默认值 | 预填在结构体里，文件没写的字段保持不变。没有 `*bool` 指针，`Enable: false` 就是 false |
| 什么时候读 | 随时。第一次读的时候框架才去找文件、加载，在 `main` 里、`xone.Run` 之前读到的也是最终值 |
| 时间 | 直接写 `30s` / `1500ms` / `1h30m`。写裸数字会启动失败——写 `30` 的人想要 30 秒，Go 的零值语义会给他 30 纳秒 |
| 字段拼错 | **启动失败**，不是静默忽略 |
| 多配了没人认领的块 | **启动失败**，并提示可能是忘了 import 对应的包 |
| `${VAR}` | 必填，未设置则启动失败。凭证类配置都该写成这个形式 |
| `${VAR:default}` | 可选，未设置时用默认值 |
| 占位符的类型 | 按替换后的内容判定，`Port: ${PORT:8080}` 进的是 int 字段。加了引号就固定按字符串处理，数字形态的密码用 `"${PW}"` |
| 占位符展开为空 | `${PORT:}` 或变量设成了空串，等于**这一项没写**：任何类型的字段都保持结构体里的默认值（注意是默认值，不是低优先级文件里的值——合并在展开之前）。真要空串就加引号：`"${PW:}"` |
| 占位符的值是 null 写法 | 变量的值恰好是 `null`、`~`、`Null`、`NULL` 时**不**当成没写，固定按字符串处理：字符串字段拿到这个字面量，其他类型的字段启动失败并报类型错误。「这一项没写」只有展开为空这一种写法 |
| 占位符与报错 | 类型不对时 yaml 的报错会带上值的前几个字符（超过 10 个字符留前 7 个）。由占位符展开出来的值在报错里换成配置里写的原文：密码填进了 int 字段，报的是 ``cannot unmarshal !!str `${DB_PASSWORD}` (expanded value redacted) into int``，凭证不会有一截进启动日志。按值认：配置里另有一个同值的字面量报错时也会被换掉 |
| 建连重试 | 连不上时按 3 次重试，两次之间的等待**逐次翻倍并带抖动**（`[0, 当前退避]` 之间取值）。退避从 **1s** 起，两次退避的上界是 1s、2s，所以启动时一个实例最多等 `3 × 单次探测预算 + 3s`（XGorm 连 PostgreSQL 默认 `3 × 1.5s + 3s = 7.5s`，XRedis 默认 `3 × 1s + 3s = 6s`）。翻倍是不想一直按同一个节奏敲正在恢复的下游，抖动是不想一群副本同时重启时在同一瞬间一起敲过去。XGorm 连 PostgreSQL 时**认证失败不重试**（SQLSTATE 第 28 类，密码错、用户不存在都是 28P01），连 MySQL 时同样（错误号 1045：密码错、用户不存在；1044：没有这个库的权限，没有全局权限的账号连一个不存在的库拿到的也是 1044；实测 MySQL 8.0.46），错误报 `authentication to <地址> failed` 而不是 `cannot reach`：服务端已经明确拒绝了，再试只是多等两轮退避。XRedis 同理（`WRONGPASS` / `NOAUTH`） |
| 超时写 0 | **不是「不限时」而是「一点都不等」**。`XTrace.ShutdownTimeout`、`XFlow.RollbackTimeout` 写 0 直接启动失败。反过来 `XGin.ReadHeaderTimeout` / `IdleTimeout` 写 0 在 net/http 里是「不限时」，同样启动失败 |
| 列表字段 | 文件里写了就整体替换，不会和默认值混在一起 |
| map 字段 | 文件里写的是**合并**进默认值，所以框架的 map 字段一律没有默认值 |

## App —— 应用身份

链路的 `service.name` / `service.version`、接口文档（XGinSwagger）的默认标题和版本都取自这里。
放在一处，免得各配一遍再对不上。指标不读这一块，`/metrics` 上没有应用名和版本：
要在指标上区分应用，用 `XMetric.ConstLabels`（如 `app: xone.demo.app`）。

```yaml
App:
  Name: xone.demo.app      # 建议 team.system.app，默认空
  Version: v1.2.0          # 默认空
```

链路的 `service.name` / `service.version` 按这个优先级取，后面的压过前面的：
OTel 自己的兜底名 `unknown_service:<可执行文件名>` → `App.Name` / `App.Version`
→ `OTEL_RESOURCE_ATTRIBUTES` → `OTEL_SERVICE_NAME`。环境变量是部署方的最后一句话，
所以压过打进镜像的配置文件。没配的那一项不写：之前的版本会写进一个空的
`service.name=""`，把兜底名和 `OTEL_SERVICE_NAME` 一起盖掉。

## XLog —— 日志

装进标准库的 `slog.Default()`，业务代码直接用 `slog.InfoContext` 即可。

```yaml
XLog:
  Level: info              # debug / info / warn / error，默认 info
  Format: json             # json / text，默认 json
  AddSource: false         # 是否记代码位置，有开销，默认关
  Timezone: ""             # 日志时间戳按哪个时区渲染，IANA 时区名，如 Asia/Shanghai
                           # 留空跟随进程本地时区（容器里通常是 UTC）
                           # 配了却加载不到直接启动失败，不会悄悄退回本地时区——
                           # 那意味着你以为在看东八区的时间、实际差八小时而毫无提示
                           # scratch / distroless 镜像里没有 /usr/share/zoneinfo，
                           # 要在自己的 main 包加一行把时区库编进去（约 400KB）：
                           #   import _ "time/tzdata"
                           # 框架不替你编：没用到这项的人不该背这 400KB
  Console: true            # 打到标准输出，由部署环境的采集组件收集，默认开
  File:
    Enable: false          # 默认关
    Path: /var/log/app     # 目录
    Name: app.log          # 实际文件是 app.log.20260918，另有同名符号链接指向当前文件
                           # 那个位置上已经有一个普通文件（不是符号链接）时启动失败：
                           # 替换过去会把里面的旧日志一声不响地吞掉，挪到哪也不该由框架替你定
    RotateTime: 24h        # 轮转周期，按本地时区对齐，默认一天，至少 1m
                           # 文件名后缀最细到分钟：0s 实际每分钟一个文件，30s 两个周期
                           # 落在同一个文件名上，所以短于 1m 直接启动失败
    MaxAge: 168h           # 历史保留时长，默认 7 天；0 表示不清理，负数启动失败
                           # 启动时清一次、之后每次轮转清一次，只删 app.log.<时间后缀>
                           # 这种自己命名的文件，app.log.bak / app.log.1.gz 一律不碰
    Perm: "0644"           # 按八进制解析的字符串，也认 644 和 0o644，引号可写可不写
                           # 不用数字是因为实测 yaml.v3 把 0644 解析成 420（对的），
                           # 漏掉前导 0 写成 644 却是十进制 644 = 0o1204，不报错
```

业务自己要格式化时间、又想和日志对得上时，用 `xlog.Location()` 取生效中的时区，
免得把配置里的时区名在代码里再抄一遍：

```go
t.In(xlog.Location()).Format(time.RFC3339)
```

## XTrace —— 链路

装好 OpenTelemetry 的全局 TracerProvider 和 Propagator，业务代码用原生的
`otel.Tracer("...")`。框架不内置任何上报 exporter（OTLP 一个就带进上百个构建依赖），
需要上报的服务自己 `xtrace.AddSpanProcessor(...)`。

```yaml
XTrace:
  Enable: true             # 默认开。关掉后 Span 是 noop，但 Header 透传照常生效
                           # AddSpanProcessor 登记的处理器收不到 Span，退出时照样被 Shutdown
  Console: false           # 把 Span 打到标准输出，本地调试用，默认关
  SampleRatio: 1           # 根 Span 的采样率，[0, 1]，越界或 NaN 启动失败
                           # 0 是「不采样但照常生成透传 TraceID」
                           # 要连 Span 都不产生请用 Enable: false，那是另一件事
                           # 有上游时一律听上游的 sampled 位，1 也不例外（ParentBased）
  ShutdownTimeout: 5s      # 退出时等导出完成的上限，默认 5s，必须 > 0
                           # 同时不超过框架给的停止预算，两者取更早的那个
  ForwardHeaders:          # 向所有下游透传的 Header，默认无
    - X-Request-Id
  ForwardHeaderRules:      # 只发给匹配域名的 Header，默认无
    - Domains: ["api.internal.com", "*.trusted.com"]
      Headers: ["X-Internal-Token"]
```

**透传只收可信对端发来的值。** 透传是「把上游给的值原样带给下游」，上游是谁就是信任边界：
照单全收的话，公网客户端发一个 `X-Tenant-Id` / `X-Internal-Token`，就被当成自己人给的，
带进内网的每一次调用。所以用 xgin 时，只有**直连的对端在 `XGin.TrustedProxies` 里**，
这些值才会被收下——「谁是自己人」只在那一处说，不另设开关。`TrustedProxies` 默认
一个都不信，于是**默认什么都不透传**；配置照样校验，写错的规则仍然启动失败。

- 判的是直连的对端（TCP 那一跳），不是从 `X-Forwarded-For` 推出来的 client IP
- **负载均衡一般会原样转发客户端发来的头**：把它写进 `TrustedProxies` 之前，
  先在它那里剥掉这些头，否则等于又信了所有客户端
- **`baggage` 同样只收可信对端的**：它和透传 Header 是同一种东西——上游给的键值原样带进
  每一次调用，不设防的话 `ForwardHeaders` 挡在门外的 `X-Tenant-Id` 改写成
  `baggage: tenant=…` 照样进了内网。本进程自己写进 ctx 的 baggage 照常带给下游
- 链路标识（`traceparent`、`b3`）不受这条影响，谁发来的都接
- 不可信的对端带着这些头来时，打一条告警（整个进程只打一次）；带着 `baggage` 来时另打一条
  `xtrace ignored baggage from an untrusted peer`，同样只打一次
- 不经过 xgin、自己调 `Extract` 的（比如从消息队列的消息头里取），carrier 要实现
  `TrustedPeer() bool` 并返回 `true` 才会被收下，否则一律当作不可信

> 行为变化：之前的版本不看来源一律收下。升级后，没配 `XGin.TrustedProxies`
> 的服务透传会停掉；确认上游可信之后，把它那一段网段写进 `TrustedProxies`。
> `baggage` 从这一版起也按这条收：原先谁发来的都收。

**透传和链路标识不跟着 `XGin.Trace` / `XHttp.Trace` 走。** 那两个开关只管开不开 Span：
关掉之后入站照样接上游的 `traceparent`、`baggage` 和透传 Header（可信规则不变），
出站照样把它们带给下游，按域名的规则照样生效。关掉的只是这一跳的 Span 和
`X-Trace-Id` 响应头。要整个进程都不产生 Span，用 `XTrace.Enable: false`。

> 行为变化：之前 `XHttp.Trace: false` 连注入一起摘掉，透传 Header 和 `traceparent`
> 断在这一跳；`XGin.Trace: false` 连入站的提取一起摘掉，上游的链路标识和透传 Header 都不收。

下面这些**读配置时就失败**（`xconfig.Unmarshal` 调 `Validate`，错误出自 xconfig、点名 `XTrace` 这一块），
不等到装 Propagator；直接调 `xtrace.New` 的，`New` 也会校验一遍。

同一个 header 同时出现在 `ForwardHeaders` 和 `ForwardHeaderRules` 里会**启动失败**：
一边说发给所有人、一边说只发给这些人，猜哪边都可能把内部标识发给第三方。

`Domains` 只认两种写法：精确的 `api.internal.com`，和通配的 `*.trusted.com`。
`*.trusted.com` 匹配任意层级的子域（`a.trusted.com`、`a.b.trusted.com`），
**不匹配裸域 `trusted.com`**——要连裸域一起就把它也写上。其余带 `*` 的写法
（`*trusted.com`、`a.*.com`、`*`）**启动失败**：`*trusted.com` 原先会匹配
`eviltrusted.com`，一个谁都能注册的域名就拿到了内部令牌。

进程与主机属性（resource）采集出错时只打一条告警、用采到的那部分继续：
`OTEL_RESOURCE_ATTRIBUTES` 写错一项（其余写对的照常生效）、或容器里以随机 UID
运行查不到当前用户，都不再让服务起不来。

## XMetric —— 指标

```yaml
XMetric:
  Namespace: myapp         # 指标名前缀，默认无
  ConstLabels:             # 附加到所有指标上（含 go_* / process_*），默认无
    env: "${ENV:dev}"
  # 两组桶不写就是默认值，写了就整体替换。下面是默认值本身：
  HTTPDurationBuckets: [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]  # 出入站 HTTP 耗时（秒）
  HistogramBuckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]           # 快捷方法建的业务直方图，即 prometheus.DefBuckets
  GoMetrics: true          # Go 运行时指标，默认开
  ProcessMetrics: true     # 进程指标，默认开
  LogErrorMetric: true     # Error 级别日志计入 log_errors_total，默认开（需配合 xlog）
```

下面这些读配置时就失败（`xconfig.Unmarshal` 调 `Validate`；直接调 `xmetric.New` 的由 `New` 校验），而不是留到运行期：

| 写法 | 不拦的话 |
|---|---|
| 桶不是严格递增（`[1, 0.5, 2]`、`[1, 1]`），或含 NaN / Inf | 通过启动，第一次 `HistogramObserve` 时在业务请求里 panic |
| 桶写成空列表 `[]` | 不是「用默认」：被 Prometheus 悄悄换成它自己的 `DefBuckets`（HTTP 那组少了 1ms 一档）。要默认值就别写这个字段 |
| `Namespace` 或 `ConstLabels` 的 key 不是「字母、数字、下划线，不以数字开头」，或标签名以 `__` 开头 | client_golang v1.24 不报错，导出时悄悄转义：实测 `my-app` 导出成 `my_app_…`，`1app` 成 `_app_…`，中文成一串下划线，看板和告警按你写的名字什么都查不到 |
| `ConstLabels` 的 key 是 `le` 或 `quantile` | 实测 `le` 通过启动，第一次 `HistogramObserve` 时在业务请求里 panic；`quantile` 让建 Summary 当场 panic |
| `ConstLabels` 的 key 撞上框架自带指标的变量标签：`level`、`caller`（`log_errors_total`），`method`、`status`、`route`、`host`（xgin / xhttp 的请求指标），`name`（xgorm / xredis 的连接池指标） | 常量标签附加在所有指标上，撞名的那个注册失败，只打一条错误日志，那组指标从此一个都导不出去 |
| `ConstLabels` 的 key 是 `version` | Go 运行时指标里的 `go_info` 自带常量标签 `version`（Go 版本）。实测撞名时 client_golang 拒绝注册整组 Go 运行时指标（`attempted wrapping with already existing label name "version"`）。要在指标上标应用版本，换个名字，如 `app_version` |

## XGorm —— 数据库

两种写法。单实例：

```yaml
XGorm:
  Driver: postgres         # mysql / postgres 内置，默认 postgres；其余驱动见下
  DSN: "${DB_DSN}"         # 必填
  DialTimeout: 500ms       # 建连超时：MySQL 注入 DSN 的 timeout，PostgreSQL 注入 connect_timeout
  MaxOpenConns: 50
  MaxIdleConns: 50         # 配 0 就是一条空闲连接都不留
  MaxLifetime: 5m
  MaxIdleTime: 5m
  Log: false               # 把 SQL 接到 slog 上，默认关（一条 SQL 一行日志）
                           # 记的是带占位符的 SQL，不含参数值：GORM 默认会把参数代进去，
                           # WHERE password = ? 就成了 password = '<真实的值>'
                           # 占位符就是发给数据库的那样：MySQL 是 ?，PostgreSQL 是 $1
                           # （GORM 的 PG 方言没参数可代时会写成 $1$，这里改回去了）
  SlowThreshold: 3s        # 超过就记 warn，需 Log 开启
  IgnoreNotFound: false    # 「没查到记录」是否不当错误
  Trace: true
  Metric: true             # 连接池指标，按实例生效：Metric: false 的实例不出现在 /metrics 里
  MySQL:                   # 仅 Driver: mysql 生效
    ReadTimeout: 3s
    WriteTimeout: 5s
  Postgres:                # 仅 Driver: postgres 生效，下列值随建连发给服务端成为会话级 GUC
    StatementTimeout: 0s   # 默认不限制；服务端的计时，管不到网络那头不回话，见下「PostgreSQL 没有读超时」
    LockTimeout: 0s
    IdleInTxTimeout: 0s
    Params: {}             # 任意 PG 运行时参数，同名时以它为准
```

**MySQL 启动时的建连全部受退出信号和重试管。** GORM 的 MySQL Dialector 在初始化时会查一次
`SELECT VERSION()`，驱动那行写死了 `context.Background()`（`gorm.io/driver/mysql` v1.6.0），
而且它就是第一次建连，发生在 `gorm.Open` 里、建连重试之前。原样用的话实测（对端收下连接却不回话）：
只试 1 次、3.0s 后直接失败，重试一次都没有；启动期间的 SIGTERM 要等握手那一读撞上
`MySQL.ReadTimeout`（不是 `DialTimeout`：TCP 秒连、握手不回话时管这一读的是读超时），
信号之后 2.95s 进程才退。这里把它关掉（`SkipInitializeWithVersion`），改在建连探测里、
每次 Ping 成功之后查：受 ctx 管、跟着重试。实测同一场景（三轮）：试满 3 次、9.6–11.1s 后失败（预算 13.5s，差别在退避的随机抖动）；
启动期间的 SIGTERM 之后 7–20ms 进程退出。版本号照样设进 Dialector，驱动靠它决定的那些行为不变——
MariaDB / MySQL 5.x 上改索引名、改列名、`FOR SHARE`、`DROP CONSTRAINT` 支不支持，
MariaDB 10.5+ 上增删改用 `RETURNING`，规则与驱动 v1.6.0 的 Initialize 逐项相同。
PostgreSQL 没有这次查询；ClickHouse 的驱动有同样的写法，同样挪进了建连探测，见下。

单次建连探测的预算：MySQL 是 `DialTimeout + MySQL.ReadTimeout`；PostgreSQL 是
「注入的 `connect_timeout`（`DialTimeout` 向上取整到整秒）+ `DialTimeout`」，
默认 1.5s——pgx 拿 `connect_timeout` 管的是每个主机的整个建连，TCP 之后的 TLS 握手、
startup、认证都在里面（pgconn v5.10.0 源码注释原话 "restricts the whole connection process"；
实测 TCP 秒连、startup 不回话的服务端，`connect_timeout=1` 等满 1.0s），
预算比它短的话，一次慢一点但合法的握手会在 pgx 放弃之前就被判超时；
其余驱动是 `2 × DialTimeout`。探测最多试 3 次，间隔见上面「通用规则」里的建连重试。
上面的 `DialTimeout`、`MySQL.ReadTimeout`、`connect_timeout` 指的都是最终生效的值：
DSN 里写了 `connect_timeout=10`（PG）、`timeout=10s` / `readTimeout=10s`（MySQL）、
`dial_timeout=10s`（ClickHouse）的，预算按驱动从 DSN 里读出来的这个值算，
不会在驱动自己放弃之前就按配置里更短的默认值判超时。

**PostgreSQL 没有读超时。** MySQL 的 `ReadTimeout` 管的是客户端等一次回包最多多久，
PG 这一侧没有对应的东西：连接池里的连接遇上不回话的对端（卡死的进程、半路断掉的网络），
调用方的 ctx 又没有截止时间，查询就一直等到调用方放弃。`StatementTimeout` 救不了这一段——
它是服务端的计时，语句没到服务端它就无从计时：实测对端吞掉全部字节、`StatementTimeout: 1s`，
一条 `SELECT 1` 30s 后仍没返回。能管住它的只有调用方的截止时间：

```go
ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
defer cancel()
xgorm.CWithCtx(ctx).First(&u, id)   // 实测给 200ms 就在 200ms 返回
```

Web 请求里用请求自带的 ctx（`c.Request.Context()`）也行：客户端断开时它被取消，
查询跟着返回。后台任务、定时任务没有这样的 ctx，**一定要自己给截止时间**。
`StatementTimeout` 另配一个值，管的是服务端执行慢（锁等待、大查询）那一段，两者互补。

**MySQL 的取消与截止时间。** 和 PG 不同，MySQL 在客户端这一侧有读超时（`MySQL.ReadTimeout`，
默认 3s）兜底：对端不回话、调用方又没给截止时间时，查询在 `ReadTimeout` 失败（实测 3.0s，
错误是 `invalid connection`），不会一直挂住。调用方的 ctx 管得比它细：

- 给了截止时间，查询就在那一刻返回——池里卡在读上的连接和主机宕机时卡在拨号上的新连接都一样（实测给 200ms 就在 200ms 返回）；
- ctx 被**取消**（客户端断开、优雅退出到点断连）时，go-sql-driver 当场关掉那条连接，阻塞在读上的查询跟着返回
  `context canceled`，不等 `ReadTimeout`：实测客户端放弃之后 0.3ms handler 就返回了；优雅退出时断连那一刻（预算 3s 时在信号后 1.6s）返回，
  不会拖到服务那一段的截止时间之后。被取消的那条连接不还回池里，下一条查询重新建连。

所以 Web 请求里传 `c.Request.Context()` 就够了；后台任务没有这样的 ctx，想比 `ReadTimeout` 更早放弃就自己给截止时间。
新建连接的拨号受 `DialTimeout`（DSN 的 `timeout`）管：主机宕机时实测 500ms 失败；
DSN 里写了 `timeout=1500ms` / `readTimeout=1s` 的，以 DSN 为准（实测 1.5s / 1.0s）。

**go-sql-driver 自己的日志**接到了 slog：它默认用 `log.New(os.Stderr, "[mysql] ", …)` 往 stderr 写纯文本
（`[mysql] 2026/09/24 10:00:00 packets.go:58 read tcp …: i/o timeout`），不是 JSON，进不了日志平台。
现在是一条 `xgorm go-sql-driver log`，级别 WARN（它的日志没有级别，写出来的都是出错时的补充：读包失败、
关掉坏连接、认证插件回退，调用方同时也拿到了错误），原文在 `detail` 字段。驱动不给 ctx，这些日志不带 trace_id。
实测对端不回话、8 条查询读超时，就是 8 条 `xgorm go-sql-driver log`，stderr 里一行 `[mysql]` 都没有。
这一步在 xgorm 的 `init` 里做（`mysql.SetLogger`），想换成自己的就在 `main` 里、xone 启动之前再调一次，
后调的那次生效。要赶在启动之前：驱动在解析 DSN 时把当时的 logger 抄进连接配置，之后再调只对新解析的 DSN 生效，
而 xgorm 在启动钩子里解析 DSN。和 xredis 一样，别放在没有 import xgorm 的包的 `init` 里：
谁先谁后取决于包路径的字典序，可能被 xgorm 盖掉。

### 其它驱动

mysql 和 postgres 内置，其余驱动住在自己的 module 里，匿名 import 一行就注册好：

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

拿到的仍然是原生的 `*gorm.DB`，配置项和多实例写法都一样。

DSN 必须是上面四种 scheme 之一的 URL，否则启动失败——包括裸的 `host:port`
（驱动自己也不认，实测 `ParseDSN("10.255.255.1:9000")` 报错）、scheme 拼错、
前面多一个空格。启动时还会用驱动自己的解析器把 DSN 过一遍（比如 `https://`
必须配 `secure=true`）。这些错误一律不回显 DSN：驱动和 `url.Parse` 的原始错误
里带着整串 DSN，连同明文密码。

驱动在初始化时查一次 `SELECT version()`，用的是写死的 `context.Background()`
（`gorm.io/driver/clickhouse` v0.7.0），而且发生在建连重试之前：实测对一个收下连接
却不回话的地址，ctx 早已取消也要等满 `dial_timeout`，然后直接失败，一次重试都没有。
这里把它关掉（`SkipInitializeWithVersion`），改在建连探测里查：受 ctx 管、跟着重试。
版本号照样设进 Dialector，驱动靠它判断老版本不支持的改列名（< 20.4）和列精度（< 21.11）。

为什么不直接放进 xgorm：实测一个只 import xgorm 的应用模块图是 65 个，
加上 ClickHouse 驱动变成 146 个（编译包 140 → 183）。多出来的大头是 Docker 和
testcontainers —— `clickhouse-go` 用它们跑集成测试，而 `go.mod` 分不出
「只测试用」。Go 的 MVS 按模块图把版本要求强加给使用者，不用它的人不该付这个钱。

驱动名写错或忘了 import 时启动会失败，错误里列出当前注册了哪些；
也可以用 `xgorm.Drivers()` 自己查。

多实例：

```yaml
XGorm:
  Clients:
    default: {DSN: "${DB_DSN}"}            # C() 取的就是这个
    report:  {DSN: "${REPORT_DSN}", MaxOpenConns: 5}
```

两种写法不能混用。DSN 里已经写了的 timeout 之类的参数不会被配置覆盖——
配置里的值只是默认值。PostgreSQL 的 key=value 形式里，默认值垫在 DSN 前面，
pgx 同一个 key 取最后一次（实测 pgconn v5.10.0），所以 DSN 里写了的自然作数，
怎么写都算（`connect_timeout = 10`、单引号括起来的值）。时区是例外：gorm 的
postgres 驱动另用正则取 DSN 里第一处 `timezone=` / `TimeZone=` / `time_zone=`，
所以 DSN 里写了时区时，`Postgres.Params` 里的时区不再垫进去。
URL 形式里补的参数接在原 query 后面，使用者写的部分原样保留、不重新编码——
同一个正则不解码，`TimeZone=Asia/Shanghai` 被编码成 `Asia%2FShanghai` 的话
每条连接都设不上时区；补进去的值里的 `/` 同样不编码。

PostgreSQL 的 DSN 预检用的是 `pgx.ParseConfig`，与 gorm 的 postgres 驱动建连时
用的同一个：它比 `pgconn.ParseConfig` 多校验 `default_query_exec_mode`、
`statement_cache_capacity`、`description_cache_capacity`，这三项写错也在预检时
报一句不带 DSN 的错。pgx 自己的错误原文是整串 DSN、只遮得住 `password=x` 这种
规整写法（实测 v5.10.0，`password = hunter2` 原样带出），所以不回传它。

PostgreSQL 多主机 URL 里 IPv6 地址不能排在第一个：`postgres://u:p@[::1]:1,h2:1/db`
会被 `url.Parse` 拒绝，pgx 自己也拒绝（实测 v5.10.0，同一个错误）。把它挪到后面
（`h2:1,[::1]:1` 可以），或者改用 key=value 形式 `host=::1,h2 port=1,1`——
这两种写法同样实测过，pgx 解得出两个主机。

## XRedis

同样支持单实例 / 多实例两种写法，多实例和 XGorm 一样写在 `Clients` 下面：

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
  DialTimeout: 500ms
  ReadTimeout: 500ms
  WriteTimeout: 500ms
  PoolSize: 0              # 0 交给 go-redis（10 × GOMAXPROCS）
  MinIdleConns: 5
  MaxIdleConns: 0          # 0 不限制
  MaxActiveConns: 0
  PoolTimeout: 1s
  ConnMaxIdleTime: 5m
  ConnMaxLifetime: 5m
  MaxRetries: 0            # 0 交给 go-redis（3 次），-1 关闭
  MinRetryBackoff: 0s      # 0 交给 go-redis（10ms），-1ns 关闭（裸写 -1 解不成时长，启动报错）
  MaxRetryBackoff: 0s      # 0 交给 go-redis（1s），-1ns 关闭；两个默认值实测 v9.22.0
  Trace: true              # Span 里只有命令名，不含参数：redisotel 默认把整条命令
                           # 连同值写进 db.statement（实测 SET 的值原样出现），这里关掉了。
                           # 钩子不是零成本：没装链路时实测每条命令约 +3µs、+8 次分配（本机回环）
  Metric: true             # 连接池指标，按实例生效，同 XGorm
```

**启动时的建连验证**：Ping 一次，单次超时 `DialTimeout + ReadTimeout`（默认 1s），
按「通用规则 · 建连重试」最多试 3 次，默认最多 3 × 1s + 3s = 6s 失败。
认证失败（服务端回 `WRONGPASS` / `NOAUTH`）**不重试**——密码不对再试也不对——
报的是 `authentication to <地址> failed`，连不上报的是 `cannot reach <地址>`，
后面跟着服务端或拨号的原文，不含凭证；原始错误用 `%w` 带着，`redis.IsAuthError(err)` 认得出来。
启动期间收到退出信号时当场放弃，不等卡在读上的那次 Ping 撞上 `ReadTimeout`。

**一条命令最多要多久**（调用方没给截止时间时）：

```
(MaxRetries+1) × (DialTimeout+ReadTimeout) + MaxRetries × MaxRetryBackoff
```

默认是 4 × 1s + 3 × 1s = 7s，`MaxRetries: -1` 时是 1s。这条式子成立是因为一次建连只拨一次号：
go-redis v9.22.0 默认在每次建连里面还藏着一层重试（`DialerRetries` 5 次、间隔
`DialerRetryTimeout` 100ms），一次「建连」就是 5 × 500ms + 4 × 100ms = 2.9s，
实测 Redis 主机宕机（SYN 没有回音）时一条命令用了 11.7s。xredis 把它固定成 1，重试只剩
`MaxRetries` 这一层，同样的场景实测默认配置 2.1s、`MaxRetries: -1` 时 0.5s。连接池累计 `PoolSize` 次建连失败之后，go-redis 改成不再拨号、
直接报上一次的错，这时一条命令只剩退避的时间。

**取消与截止时间**：命令听的是 ctx 的**截止时间**，不是取消。go-redis 把
`ctx.Deadline()` 设成 socket 的读写截止时间之后就不再看 ctx，ctx 被取消叫不醒一个已经
阻塞在读上的命令，它照样等到 `ReadTimeout`（实测 v9.22.0：`ReadTimeout: 1s`、100ms 时取消，
命令 1.0s 返回）。这件事 xredis 在里面补不上：命令的结果写在调用方拿着的 `*Cmd` 上，提前返回
就是和还在读的那个协程抢着写同一个对象。落到停止流程上：XGin 到点断连时取消了请求的 ctx，
卡在 Redis 读上的 handler 不会因此返回，要等 `ReadTimeout`，或者等 xredis 的停止钩子关掉
连接池（实测 `ReadTimeout: 10s`、`WithStopTimeout(3s)`：1.6s 断连，handler 到 2.0s
连接池关掉时才返回，`Stop` 报 `1 handler(s) still running`）。要让 Redis 调用在停止时
停得下来，给 ctx 带上截止时间（`context.WithTimeout`），或者让 `ReadTimeout` 短于
XGin 断连之后留出来的那一截。

**go-redis 自己的日志**接到了 slog：它默认用标准库 log 往 stderr 写纯文本
（`redis: 2026/09/24 10:00:00 pool.go:762: ...`），不是 JSON、不带 trace_id。
现在是一条 `xredis go-redis log`，级别 WARN（它的日志没有级别，写出来的几乎都是出错时的补充，
调用方同时也拿到了错误），原文在 `detail` 字段。ctx 原样传下去，用命令 ctx 记的那些带着
请求的 trace_id；最常见的那句 `failed to dial` 不带——go-redis v9.22.0 在它自己的协程里、用
`context.Background()` 拨号，实测 Redis 拒绝连接时 45 条一条都没有 trace_id。这一步在 xredis 的 `init` 里做，想换成自己的就在 `main` 里再调一次
`redis.SetLogger`，后调的那次生效。自定义的 logger 请在 `main` 里、或在一个 import 了 xredis 的包里设置：
Go 保证被依赖的包先初始化，这两处都晚于 xredis 的 `init`；放在没有 import xredis 的包的 `init` 里，
谁先谁后取决于包路径的字典序，可能被 xredis 盖掉。

## XCache —— 本地缓存

```yaml
XCache:
  NumCounters: 1000000     # 频率计数器个数，建议取预期条目数的 10 倍
  MaxCost: 100000          # 总成本上限。本包写入时 cost 固定为 1，所以等价于条目数
                           # 统计的是使用者给的 cost，不含 ristretto 每条 56 字节的
                           # 内部开销（它默认是算进去的，那样 MaxCost: 2000 实际只
                           # 存得下 35 条）。按字节记 cost 时把这部分算进自己的预算
  BufferItems: 64
  DefaultTTL: 5m           # 包级 Set 用的过期时间。0 是永不过期；负数启动失败
                           # （ristretto 会把 ttl<0 的写入直接丢掉，一条都存不进去）
```

同样支持多实例，写法和 XGorm 一样：`XCache: {Clients: {hot: {...}, cold: {...}}}`。

`xcache.DefaultTTL("name")` 名字写错时和 `C("name")` 一样 panic，不返回 0——
0 在 ristretto 里是「永不过期」。

停止时**不调 ristretto 的 `Close`，只调 `Clear`**。ristretto v2.4.2 的 `Close`
先关内部 channel、最后才标记已关闭，和它并发的 `Set` / `Del` / `Get` 会直接 panic
（实测 100 轮并发读写里 750 个协程 panic）。你拿到的是原生 `*ristretto.Cache`，
停止钩子跑的时候谁还攥着它，框架不知道。`Clear` 对并发读写是安全的，并把条目全部放掉；
代价是 ristretto 的两个后台协程和计数器内存（默认 `NumCounters` 约 4.3MB）
留到进程退出。自己用 `xcache.New` 建、并确定调用方都已停下的，可以直接调
`cache.Close()` 把这部分也收回来。

## XHttp —— 出站 HTTP

不配也能用，默认值就是一份合理的配置。

```yaml
XHttp:
  Timeout: 60s             # 一次尝试的超时。配 0 是「永不超时」，不是「用默认值」
  DialTimeout: 30s
  DialKeepAlive: 30s
  MaxIdleConns: 100
  MaxIdleConnsPerHost: 10  # 标准库默认只有 2，对只调几个下游的服务太小
  IdleConnTimeout: 90s
  RetryCount: 0            # 默认不重试
  RetryWaitTime: 100ms
  RetryMaxWaitTime: 2s
  RetryOnlyIdempotent: true  # 默认只重试幂等方法，见下
  Trace: true              # 出站 Span。只管 Span：关掉后 traceparent、baggage、
                           # 透传 Header 照样带给下游，见 XTrace 那一节
  Metric: true             # 出站耗时指标，见下
```

配错的值（负的时长、负的重试次数和连接数）在读配置时就失败，直接调 `xhttp.New` 的由 `New` 校验。

`Metric` 开着时导出 `http_client_request_duration_seconds`（带 XMetric 的 `Namespace` 前缀），
标签三个：

| 标签 | 取值 | 基数 |
|---|---|---|
| `method` | 收敛到固定集合：`GET` `HEAD` `POST` `PUT` `PATCH` `DELETE` `CONNECT` `OPTIONS` `TRACE`，其余（包括小写的 `get`）一律 `OTHER` | 最多 10 |
| `host` | 请求 URL 里的 `host[:port]`，原样照抄 | **等于你调过的目标数**，见下 |
| `status` | 响应状态码；没拿到响应（建连失败、超时）记 `0` | 有界 |

`host` 没法收敛：它就是「打给谁」，看板要靠它分下游。它的基数由业务决定——
调固定几个下游时是个位数；**目标地址来自用户输入、按租户拼子域名、或者直连一批 IP**
（`http://10.0.3.17:8080`）时，每个新值都要乘上它出现过的 `method` × `status` 组合，
每个组合是 15 条时间序列（默认 12 个桶，加 `+Inf`、`_sum`、`_count`），没有淘汰机制。那种调用关掉 `Metric` 用单独的客户端发
（`xhttp.New` 自己建一个），或者在前面挂一层固定域名的网关。

指标注册失败（比如同名指标已被别处注册成别的类型）**不让启动失败**：只打一条
`xhttp failed to register the request duration metric` 的错误日志，客户端照常可用，
那组指标导不出去——与 xgin / xgorm / xredis 一致，可观测性的问题不该让出站调用跟着起不来。

`RetryOnlyIdempotent` 默认开着：传输层超时分不出「请求没到服务端」和
「服务端处理完了但响应丢了」，重发一个 POST 就可能变成重复下单。
确认接口幂等（比如带幂等键）之后再关掉它。
重试什么和 resty 自己的默认一致，只是多了一道方法的限制：只重试传输层的错
（建连失败、超时、连接被重置、响应体没收全），拿到了响应就不重试——5xx 也不重试，
响应完整收到之后的 JSON 解析失败更不重试。挂上重试条件会让 resty 自己的判断整个作废
（resty v2.17.2），所以这条是 xhttp 自己守着的：从前它漏了，实测 200 + 坏 JSON、
`RetryCount: 3` 时同一个 GET 发了 4 次，不挂条件的 resty 只发 1 次。

`Timeout` 管的是一次尝试，不是一次逻辑请求。开了 `RetryCount` 之后最坏情况是
`(RetryCount+1) × Timeout` 再加上几次退避等待——`Timeout: 300ms` 配
`RetryCount: 3`，实测整整跑了 1.24s。要给整次逻辑请求封顶，用调用方的 ctx，
每次尝试和中间的退避都听它的：

```go
ctx, cancel := context.WithTimeout(ctx, 500*time.Millisecond)
defer cancel()
resp, err := xhttp.C().R().SetContext(ctx).Get(url)
```

几处跟 resty / otelhttp 默认行为不一样的地方（都是量过的）：

| | 库自己的默认 | 这里 |
|---|---|---|
| resty 的日志 | 写 `os.Stderr`；开了重试后每次失败打一行 `WARN RESTY Get "http://…?token=…": …, Attempt 1`，用完再打一行 ERROR | 接到 slog（消息 `xhttp resty log`，内容在 `detail` 字段），级别照搬，URL 去掉查询串 |
| 出站 Span 名 | 我们原先用 `GET /users/42`，基数随 id 增长 | 只用方法 `GET`（OTel 语义约定在没有路由模板时的写法），目标看 `url.full` / `server.address` |
| `url.full` 属性 | otelhttp v0.71 只去掉 `user:password`，查询串原样写进去 | 查询串和片段一并去掉 |
| cookie jar | `resty.New()` 自带一个，同一 client 的所有请求共享会话 cookie | 没有。初始化前 / 关闭后的兜底实例也没有，与配置出来的实例一致 |

## XGin —— Web 服务

```yaml
XGin:
  Host: "0.0.0.0"
  Port: 8080
  Mode: release            # release / debug / test，默认 release
  UseH2C: false            # 非 TLS 下启用 HTTP/2。TLS 模式下 HTTP/2 本来就是自动的
                           # 只认「先验知识」：客户端一上来就发 HTTP/2 前言（gRPC、
                           # curl --http2-prior-knowledge）。HTTP/1.1 的 Upgrade: h2c
                           # 握手不支持，发它的客户端拿到的是普通 HTTP/1.1 响应。
                           # 用的是标准库的 Protocols，h2c 连接和 HTTP/1.1 一样受
                           # 优雅退出管：Shutdown 等在途请求做完，超时后 Close 断开
  CertFile: ""             # 与 KeyFile 必须同时配或同时留空，只配一半会启动失败
  KeyFile: ""
  ReadHeaderTimeout: 10s   # 慢连接攻击的主要防线，必须 > 0
                           # 0 不是「用默认值」：net/http 会退到 ReadTimeout（默认 0），
                           # 结果是不限时——发半个请求头的连接一直不被断开（实测）
  ReadTimeout: 0s          # 默认不限制：限制它会打断大文件上传。负数启动失败
  WriteTimeout: 0s         # 默认不限制：限制它会打断 SSE、长轮询、大文件下载。负数启动失败
  IdleTimeout: 60s         # 必须 > 0，理由同 ReadHeaderTimeout：0 等于空闲连接永不回收
  MaxMultipartMemory: 8388608
                           # 字节，默认 8MB。解析 multipart 表单时在内存里留多少。
                           # 不是「请求体上限」，是「超过多少才落盘」：
                           # 超出的部分写进临时文件，不会被拒绝。
                           # 实际代价约是这个数的三倍——一次 60MB 的上传，
                           # 配 32MB 时解析这一步让堆多占 96MB，配 8MB 是 24MB。
                           # gin 自己默认 32MB，二十个并发上传就是两个 G
  TrustedProxies: []       # 信任哪些代理发来的 X-Forwarded-For / X-Real-IP，默认一个都不信
                           # gin 自己的默认是「全都信」，那样任何人发一个
                           # X-Forwarded-For 就能决定访问日志里的 client_ip 是什么，
                           # 建在这个字段上的限流和审计跟着一起失效。
                           # 真在负载均衡后面时写它那一段网段：["10.0.0.0/8"]
                           # 写错的网段会直接启动失败，不会只生效一半
                           # 它同时决定收不收透传 Header（XTrace.ForwardHeaders）和 baggage，
                           # 只收直连对端在这张表里的，见 XTrace 那一节

  # 内置中间件的开关
  Log: true                # 访问日志
  LogSkipPaths: []         # 不记访问日志的路径：以 / 结尾的按前缀匹配，其余精确匹配。
                           # Metric 开着时指标端点会自动加进来，不用自己写
  LogRequestBody: false    # 请求体进访问日志，默认关，见下
  LogResponseBody: false   # 响应体进访问日志，默认关
  Trace: true              # 链路：每个请求一个服务端 Span，响应头回带 X-Trace-Id
                           # 只管 Span：关掉后照样接上游的 traceparent、baggage、
                           # 透传 Header，只是不开 Span、不回带 X-Trace-Id
  Metric: true             # 请求指标，并自动挂上 MetricPath 这个端点
  MetricPath: /metrics     # Metric 开着时必须以 / 开头，否则启动失败。gin 不会拒绝别的写法，
                           # 而是悄悄改写（实测 v1.12.0）：metrics 注册成 /metrics，
                           # 访问日志却跳不过它；留空则挂在根路径 / 上，
                           # 业务再注册首页时 gin 在业务自己的路由代码里 panic
  ZHTranslations: false    # validator 的报错翻成中文，用法见 xgin/trans
```

停止没有单独的超时。`Stop` 等在途请求做完，最多等到它收到的 ctx 的截止时间。在 `xone.Run` 里
这个截止时间就是服务那一段停止预算——`xone.WithStopTimeout`（默认 15s）的 2/3，也就是 10s。
整个退出流程只有这一份预算，要调就调它。

到点还没做完的请求分两步收场：

1. `Shutdown` 只用到截止时间前的一截（留出剩余时间的 20%，最多 1s），到那时还有请求就 `Close()`
   断开所有连接。`Shutdown` 超时只返回错误、不动那些连接，不补这一刀的话请求会一直跑下去。
2. 断开连接**不等于 handler 返回了**：`Close()` 只关连接、取消请求的 ctx，handler 所在的协程
   照跑。所以留出来的那一截用来等 handler 真正返回，看到 ctx 取消就收尾的 handler 在这里做完。

框架保证的是：`Stop` 返回 nil 时所有 handler 都已经返回；到截止时间还有 handler 没返回时，
返回的错误里写明还剩几个（`N handler(s) still running when the shutdown deadline passed`）。
框架保证不了的是让一个**不看 ctx 的 handler** 停下来——Go 没有从外面终止协程的办法，它会在
框架关掉数据库之后继续跑。handler 里的慢操作（查库、调下游）要传 `c.Request.Context()`，
断连之后才停得下来。Redis 是例外：go-redis 只认 ctx 的截止时间、不认取消，
断连叫不醒卡在 Redis 读上的 handler，见 XRedis 一节「取消与截止时间」。被劫持走的连接（WebSocket）不归 `Shutdown` / `Close()` 管，它的 handler
同样算在「还没返回」里。

单独用 xgin、不经过 `xone.Run` 时，截止时间由你传给 `Stop` 的 ctx 定，比如
`context.WithTimeout(ctx, 10*time.Second)`。不带截止时间的 ctx 会一直等到在途请求全部做完
（实测一个 1.5s 的请求，`Shutdown` 等了 1.57s 才返回），挂住的请求会让 `Stop` 一直不返回。

> 行为变化：`ShutdownTimeout` 已经删掉，配置里还写着它会以「字段不认识」启动失败，删掉那一行即可。
> 它原来和服务那一段预算取更早的截止时间，而两者默认都是 10s，所以经 `xone.Run` 时默认行为不变；
> 配得比 10s 小的，改用 `xone.WithStopTimeout` 把整份预算调小。单独调 `Stop` 的，原来还有它兜底，
> 现在由你给的 ctx 决定。

几条容易绊倒的：

- **什么时候都读得到最终配置。** XGin 块在装配（`Engine()` 或 `Start`）那一刻才读，而配置文件
  第一次有人读的时候才加载——在 `main` 里、`xone.Run` 之前调 `Engine()`，拿到的 engine 用的也是
  最终配置。装配只有一次，之后的 `WithConfig` / `WithRoutes` 不再生效。配置不合法时照样给一个按
  默认值装好的 engine（不信任何代理、8MB 的 multipart 阈值）并记一条告警；错误由 `xone.Run`
  在启动时报出来，单独调 `Start` 的由它返回，不监听。
- **`WithRoutes` 回调里的设置以回调为准。** 回调在配置落到 engine 上之后才跑，在里面调
  `SetTrustedProxies`、改 `MaxMultipartMemory` 会盖过配置。例外是透传 Header：它认的可信对端
  只看配置里的 `TrustedProxies`（gin 不暴露它当前的代理列表），两边要一起改就改配置。
- **`Mode` 是进程级的。** `gin.SetMode` 没有实例级的版本，一个进程里用 `WithConfig`
  起两个 Mode 不同的服务，后装配的那个说了算。多实例时让它们的 Mode 一致。

同一个进程里的第二个服务（比如只听本机的管理端口）用 `WithConfig`，传的是完整配置，不是补丁：

```go
c := xgin.CurrentConfig()          // 配置文件里的 XGin 块，什么时候调都行
c.Host, c.Port = "127.0.0.1", 9090
admin := xgin.New().WithConfig(c).WithRoutes(adminRoutes)
```

框架不替业务定请求体上限，那得按接口来。要限的话在中间件里：

```go
gx.WithMiddleware(func(c *gin.Context) {
    c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 10<<20)
    c.Next()
})
```

请求/响应体日志（`LogRequestBody` / `LogResponseBody`）默认关闭。打开它意味着每个请求
都要缓存一份 body、逐字段脱敏，而漏配一个字段名就是凭证明文落盘。打开前用
`middleware.AddSensitiveFields(...)` 把业务自己的敏感字段补上。

脱敏按**敏感词**匹配，不是按字段名精确匹配：比较前双方都转小写、去掉 `_ - .` 和空格，
字段名里**含**任一敏感词就遮。所以 `password` 能认出 `new_password`、`user.password`、
嵌套在数组里的 `old-password`，`token` 能认出 `sessionToken`、`X-Csrf-Token`。

| | 规则 |
|---|---|
| 默认敏感词 | `password` `passwd` `secret` `token` `authorization` `apikey` `accesskey` `privatekey` `credential` `cookie` `session` `signature` |
| body（JSON / 表单） | 键含敏感词就遮这个值，任意嵌套层级都算。其余的值原样写回：大整数不丢精度，`<` `>` `&` 不被转义 |
| 其它 body（纯文本、XML…） | 定位不了字段，出现敏感词就整个遮掉 |
| 请求头 | 名字在名单里（`Authorization` `Proxy-Authorization` `Cookie` `Set-Cookie` `X-Api-Key` `X-Auth-Token`，加上 `AddSensitiveHeaders` 追加的）**或者**名字含敏感词，就遮 |

大小写按 Unicode 折叠比较，与 `encoding/json` 匹配字段名的规则一致：实测 `{"ſecret":…}`
（长 s，U+017F）绑得上 `Secret`，`{"toKen":…}`（开尔文符号，U+212A）绑得上 `Token`，
这样的键同样被认成敏感词。

`AddSensitiveFields` 追加的是敏感词，body 和请求头一起生效；`AddSensitiveHeaders`
追加的是精确的头名，用于名字里没有敏感词、内容却是凭证的头（比如 `X-Tenant-Id`）。

词表故意不收太短、太泛的词：`auth` 会误中 `author`，`key` 会误中 `Idempotency-Key`、
`Sec-WebSocket-Key`，`pwd` 会在随机串里撞上、把整段纯文本遮掉。

访问日志里另外几条：

- `path` 不带查询串。值是 URL 的请求头——`Referer`，以及反向代理转发原始 URI 用的
  `X-Original-URL` `X-Original-URI` `X-Rewrite-URL` `X-Forwarded-URI`——同样去掉 `?` 和 `#`
  之后的部分：OAuth 回调的 `?code=`、链接上的 `?token=` 不会从这里绕进日志。
- `multipart/form-data` 和 `application/octet-stream` 的请求体不读，只记一句 `omitted`。
  Content-Type 大小写不敏感，`Multipart/Form-Data` 一样不读。
- handler 以 `http.ErrAbortHandler` 中止的请求（`httputil.ReverseProxy` 转发到一半上游断开时
  也是这样），访问日志、指标、链路里的状态码记成 **499**，链路标为错误，访问日志的 `errors`
  里带着 `net/http: abort Handler`。这个码不会发给客户端——连接直接断了——而此前记下的是
  已经发出去的那个 200。

## XGinSwagger

单独一个 module，UI 资源不进普通服务。

```yaml
XGinSwagger:
  Host: api.example.com    # 文档里显示的地址
  BasePath: /api/v1
  Title: ""                # 默认取 App.Name
  Description: ""
  Schemes: []              # 默认留空：沿用注解里的 @schemes，写了才覆盖
  URLPrefix: ""            # UI 挂载路径前缀，默认挂在 /swagger/*any
                           # 留空，或以 / 开头、不以 / 结尾，否则读配置时就失败
```

`Register` 什么时候调都行（比如 `main` 顶上就调了 xgin 的 `Engine()`）：
配置第一次读时才加载，读到的永远是最终值，`URLPrefix` 和元信息都照样生效。
标题、版本默认取自 `App`，它们在第一次访问文档时才填——那时 xapp 一定读好了。
没写的字段一律沿用注解里的值，写了才覆盖。

> 行为变化：`Schemes` 原先默认 `["https", "http"]`，而默认值是预填进结构体的，
> 于是配置里没写它也会盖掉注解里的 `@schemes`。现在默认留空；原来依赖那个默认值的，显式写上即可。

## XFlow —— 流程编排

```yaml
XFlow:
  Monitor: true            # 关掉之后 Execute 一次监控回调都不走
  RollbackTimeout: 30s     # 回滚全部步骤的总预算，必须大于 0
```

回滚不沿用调用方的 context，否则请求一超时补偿必然全部失败——而补偿最需要
执行的时机恰恰就是那时候。它由 `RollbackTimeout` 单独限时。

这份预算对不看 ctx 的 `Rollback` 同样有效：到点就不再等它，那一步记进
`RollbackErrors`（错误里带 `context.DeadlineExceeded`），没轮到的步骤也逐个记下，
`Execute` 随即返回。被放弃的那一步的协程**不会被杀掉**，仍在后台跑、仍可能读写
`data`——`Rollback` 里该看 ctx 的还是要看。

---

## 写一个自己的集成

第三方集成只需要认识两个包：`xhook`（登记启动 / 停止钩子）和 `xconfig`（读配置）。

最小形态（单实例、无外部资源）：

```go
const ConfigKey = "XMine"

type Config struct {
    Addr string `yaml:"Addr"`
}

func DefaultConfig() Config { return Config{Addr: "127.0.0.1:1234"} }

func New(ctx context.Context, c Config) (*Client, io.Closer, error) { /* 纯构造，不碰全局 */ }

// 接入框架就这两个钩子，里面写普通的 Go 代码
func init() {
    // 被业务依赖的客户端声明 StageClient；业务代码自己的钩子不写档位
    xhook.BeforeStart(initXMine, xhook.At(xhook.StageClient))
    xhook.BeforeStop(closeXMine, xhook.At(xhook.StageClient))
}

func initXMine(ctx context.Context) error {
    c := DefaultConfig()
    if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
        return err
    }
    client, closer, err := New(ctx, c)
    if err != nil {
        return err
    }
    setDefault(client, closer) // 存到哪、怎么取，是你自己的事
    return nil
}

func closeXMine(context.Context) error { return currentCloser().Close() }
```

`xconfig.Unmarshal` 把「默认值 + 配置文件覆盖」的结果填进你的结构体。整块没配时
结构体原样不动，要区分「没配」和「配了」用 `xconfig.Has`——问过这一块就算你认领了它，
所以「没配就跳过」的包不会因为没读而被当成没人认领。字段要写 `yaml:"字段名"` 的 tag：
yaml.v3 对没写 tag 的字段只认全小写的 key（字段 `Endpoint` 认 `endpoint`、不认 `Endpoint`），
忘了写时报错里会直接说该加哪个 tag。认不出的字段是错误；
结构体实现了 `Validate() error` 的话解完会调一次——`xflow` 就是靠这条拦住
`RollbackTimeout: 0` 的，那种配错不会让任何初始化失败，只会让补偿全被跳过。

`Unmarshal` **什么时候调都行**：第一次调用时框架才去找配置文件、加载它，
读到的永远是最终值，不会因为读得早就静默拿到一份默认值。

**档位**用 `xhook.At(...)` 指定，不写就是 `StageBusiness`。只有必须早于或晚于别人的
才需要：`StageLog`（最先起、最后关）、`StageTelemetry`（链路与指标，要早于客户端）、
`StageClient`（数据库、缓存、HTTP 客户端，被业务依赖）、`StageBusiness`（使用者自己的
业务资源，默认档）、`StageServer`（最后起、最先关）。停止钩子是启动的整体镜像，所以
一个资源只声明一次档位就管住了两头。同档内按 Go 初始化包的顺序执行，稳定但不由
import 语句的书写顺序决定，有先后要求的放进不同档位。

**启动失败时框架不会调你的停止钩子**——一起登记的就是一对：停止钩子只在同一个包里、
在它之前最近登记的那个启动钩子成功之后才执行，所以停止钩子里不必处理「资源还没建起来」。

要支持「单实例 / 多实例两种写法」就再加两样：

```go
// 配置分派。两种写法里的每个实例都先铺上 DefaultClientConfig 再解，
// 文件里没写的字段保持默认，ClientConfig 不必自己写 UnmarshalYAML
func (c *Config) UnmarshalYAML(n *yaml.Node) error {
    clients, err := xconfig.DecodeClients(n, DefaultClientConfig)
    if err != nil {
        return err
    }
    c.Clients = clients
    return nil
}

// 建实例的地方从「一个」变成「按名字挨个建」。
// 存到哪、怎么取还是本包自己的事——下面是最直白的做法，一个加锁的 map
var (
    mu    sync.RWMutex
    items map[string]*Client
)

func initXMine(ctx context.Context) error {
    c := DefaultConfig()
    if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
        return err
    }
    // 按名字排序挨个建，中间有一个建不起来就把已经建好的全关掉再报错，
    // 并在错误里点名是哪一个：启动钩子返回错误时框架不会调本包的停止钩子，
    // 不自己收拾就会漏掉那几个连接池
    return buildAll(ctx, c.Clients)
}

func C(name ...string) *Client { ... }
func Has(name ...string) bool  { ... }
func Names() []string          { ... }
```

本仓库的 `xgorm` / `xredis` / `xcache` 共用一份内部实现（`internal/xclient`），
它是 internal 的：那是三个模块的共用代码，不是使用者要学的东西。
你自己的集成用上面那个加锁的 map 就够了，不值得让所有人为它多认一个包。

两条硬性要求（`check.sh` 会查）：

- 必须导出纯构造器 `New`——零装配是默认路径，不能是唯一路径
- 不许 import 根包，只能 import `xhook` / `xconfig`：依赖是单向的

---

## 业务自己的配置块

配置文件里出现一个没人读的顶层 key，进程会**直接起不来**（多半是拼错了，
或者忘了 import 对应的包）。所以业务加了自己的配置块，就要有人读它。

`xconfig.Unmarshal` 什么时候调都行，所以最简单的写法是在 `main` 里读，
把值交给你的 Runnable：

```go
// internal/conf/conf.go
package conf

import (
    "time"

    "github.com/xiaoshicae/x-one/xconfig"
)

type Config struct {
    Topic          string        `yaml:"Topic"`
    Workers        int           `yaml:"Workers"`
    MessageTimeout time.Duration `yaml:"MessageTimeout"`
}

// 默认值预填在结构体里，文件里没写的字段保持不变
var conf = Config{Topic: "orders", Workers: 4, MessageTimeout: 5 * time.Second}

// Load 读出 MyApp 这一块。main 里调一次，之后 C() 处处可用
func Load() error { return xconfig.Unmarshal("MyApp", &conf) }

func C() Config { return conf }
```

```go
func main() {
    if err := conf.Load(); err != nil {
        log.Fatal(err)
    }
    c := conf.C()
    xone.MustRun(&Consumer{Workers: c.Workers, Timeout: c.MessageTimeout})
}
```

```yaml
MyApp:
  Topic: orders
  Workers: 4
  MessageTimeout: 5s
```

业务的配置块和框架的走的是同一套规则：默认值预填、字段拼错启动失败、
`${VAR}` 占位符照样生效。读的时候框架才去找配置文件、加载它，所以这里读到的
和各集成在启动钩子里读到的是同一份。

可运行的完整例子在 `example/consumer/`。
