# 配置参考

一个 YAML 文件，框架统一读，按顶层 key 分发给各组件。这份文档讲**所有配置块共用的规则**：文件位置、Profile、
Import、合并、占位符。每个配置块的全部字段、默认值和最要紧的几条规则在它所属模块的 README 里，
见[配置块 → 模块文档](#配置块--模块文档)。为什么这样定、量出来的数字在 [`behavior.md`](behavior.md) 和各模块 README；
在代码里读自己的配置块见 [xconfig](../xconfig/README.md)，其余用法在 [`guide.md`](guide.md)。

最常用的几件事：

| 想要 | 怎么写 | 详见 |
|---|---|---|
| 指定配置文件 | 不指定就找 `conf/application.yml`；要换用 `--config=<path>` 或 `XONE_CONFIG` | [文件位置与优先级](#文件位置与优先级) |
| 按环境分文件 | 差异写进 `application-prod.yml`，`--profile=prod` 或 `XONE_PROFILE=prod` 选 | [Profiles](#profiles--按环境分文件) |
| 凭证不进版本库 | `Password: "${DB_PASSWORD}"`，没设就启动失败；可选的写 `${VAR:默认值}` | [占位符](#占位符) |
| 拆成几个文件 | `Import: [shared.yml, optional:local.yml]` | [Import](#import--引入别的配置文件) |
| 读自己的配置块 | `xconfig.Unmarshal("MyApp", &c)` | [xconfig](../xconfig/README.md) |

**import 了哪个集成，它就生效**，没写的块全用默认值——`XLog`、`XTrace`、`XMetric`、`XHttp`、`XGin`
不配也照常工作。例外是 **`XGorm`、`XRedis`、`XCache`**：它们要连的东西只有你知道，
没配这一块就一个实例都不建，`C()` 会 panic 并说明「没配」。

- [文件位置与优先级](#文件位置与优先级)
- [Profiles —— 按环境分文件](#profiles--按环境分文件)
- [Import —— 引入别的配置文件](#import--引入别的配置文件)
- [合并规则](#合并规则)
- [占位符](#占位符)
- [通用规则](#通用规则)
- [编辑器补全](#编辑器补全)
- [配置块 → 模块文档](#配置块--模块文档)

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

XGorm（PostgreSQL / MySQL / ClickHouse）、XRedis、XHttp 连出去时的 TLS 都写成同一个块（`TLS:`），
见 [xtls](../xtls/README.md#配置)。

## 编辑器补全

仓库根目录的 `config_schema.json` 是各模块 README「配置」一节的机器可读版本，挂上之后 YAML 里就有字段补全、拼错标红和悬停说明：

```jsonc
// .vscode/settings.json —— JetBrains 在 Settings → JSON Schema Mappings 里配
{ "yaml.schemas": { "./config_schema.json": ["conf/application*.yml"] } }
```

它由 `go run ./internal/schemagen` 从 Config 结构体生成，字段说明取自结构体上的注释。块里不认识的字段标红；
单实例 / 多实例两种写法都认、混着写标红；数字和布尔字段也收占位符。**顶层**不认识的 key 不标红——
业务自己的配置块也写在顶层，那由启动时的「没人读的顶层块」拦住。

## 配置块 → 模块文档

每个配置块的全部字段、默认值和要点在它所属模块 README 的「配置」一节；同一个 README 里还有这个模块的实测、
日志 / 指标 / Span 和排错。

| 配置块 | 是什么 | 模块文档 |
|---|---|---|
| `App` | 应用身份 | [xapp/README.md](../xapp/README.md#配置) |
| `XLog` | 日志 | [xlog/README.md](../xlog/README.md#配置) |
| `XTrace` | 链路 | [xtrace/README.md](../xtrace/README.md#配置) |
| `XMetric` | 指标 | [xmetric/README.md](../xmetric/README.md#配置) |
| `XGorm` | 数据库 | [xgorm/README.md](../xgorm/README.md#配置) |
| `XGorm`（`Driver: clickhouse`） | ClickHouse 驱动 | [xgorm/clickhouse/README.md](../xgorm/clickhouse/README.md#配置) |
| `XRedis` | Redis | [xredis/README.md](../xredis/README.md#配置) |
| `XCache` | 本地缓存 | [xcache/README.md](../xcache/README.md#配置) |
| `XHttp` | 出站 HTTP | [xhttp/README.md](../xhttp/README.md#配置) |
| `XGin` | Web 服务 | [xgin/README.md](../xgin/README.md#配置) |
| `XGinSwagger` | 接口文档 | [xginswagger/README.md](../xginswagger/README.md#配置) |
| `XFlow` | 流程编排 | [xflow/README.md](../xflow/README.md#配置) |
| `TLS`（XGorm / XRedis / XHttp 块里的） | 客户端 TLS | [xtls/README.md](../xtls/README.md#配置) |
