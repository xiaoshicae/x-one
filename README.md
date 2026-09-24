# xone

Go 三方库集成框架。你 import 想用的集成包，写一个配置文件，调一次 `xone.Run`：
框架统一读配置、按阶段初始化各组件、收到退出信号后逆序关闭。
业务代码拿到的是**原生 client**——`*gorm.DB`、`*redis.Client`、`*gin.Engine`、标准库 `slog`——
不用学一套包装过的 API，也没有装配样板。

- **配置驱动**：行为写在 YAML 里，默认值预填，拼错字段直接启动失败
- **按阶段启动、逆序关闭**：日志 → 链路/指标 → 客户端 → 业务 → 服务，关闭反过来
- **用什么付什么**：每个带三方依赖的集成是独立的 Go module，不用的不进你的模块图

## 安装

核心（Go 1.22+）加上你要的集成。每个集成是**独立的 module**，要分别 `go get`
（集成需要 Go 1.25+）：

```bash
go get github.com/xiaoshicae/x-one                 # 核心：Run、xhook、xconfig、xlog、xflow、xapp
go get github.com/xiaoshicae/x-one/xgin            # 按需：xtrace xmetric xgorm xredis xcache xhttp xgin xginswagger
```

> 各模块还没打 tag（首个版本 v0.1.0 待发布）。在那之前请用本仓库的 `go.work`
> 或 `replace` 指向本地路径。

## 快速开始

```go
// main.go
package main

import (
	"log/slog"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xgin"

	_ "github.com/xiaoshicae/x-one/xlog" // 想要哪个组件就匿名 import 哪个，书写顺序无所谓
)

func main() {
	xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
		e.GET("/hello", func(c *gin.Context) {
			slog.InfoContext(c.Request.Context(), "hello called") // 标准库 slog，已按 XLog 配好
			c.JSON(http.StatusOK, gin.H{"msg": "hello"})
		})
	}))
}
```

```yaml
# conf/application.yml
XLog:
  Level: info
XGin:
  Port: 8080
```

```bash
go run .                          # 自动找到 conf/application.yml
curl localhost:8080/hello         # {"msg":"hello"}；访问日志、/metrics 由 xgin 自动挂上
# Ctrl+C 优雅退出
```

更完整的可运行示例在 [`example/`](example/)（各自是独立 module，进目录跑）：

```bash
cd example && go run . --config=application.yml            # Web 服务：日志、链路、指标、缓存
cd example/consumer && go run . --config=application.yml   # 消息队列消费者（非 Web）
cd example/component && go run . --config=application.yml  # 自己写一个集成
```

## 模块一览

| 模块 | 给你什么 | 怎么用 | 配置 |
|---|---|---|---|
| `xlog`（核心） | 基于 `log/slog` 的日志；装了 `xtrace` 时自动带 `trace_id` | 直接用 `slog.InfoContext(ctx, ...)` | [XLog](docs/config.md#xlog--日志) |
| `xapp`（核心） | 应用名、版本 | `xapp.Name()` / `xapp.Version()` | [App](docs/config.md#app--应用身份) |
| `xflow`（核心） | 流程编排，失败自动回滚 | `xflow.New[T](name, steps...).Execute(ctx, data)` | [XFlow](docs/config.md#xflow--流程编排) |
| `xtrace` | OpenTelemetry 链路，设为全局 TracerProvider | `otel.Tracer("app").Start(ctx, "op")` | [XTrace](docs/config.md#xtrace--链路) |
| `xmetric` | Prometheus 指标 | `xmetric.Timer("op")()`、`xmetric.CounterInc(...)`、`xmetric.Registry()` | [XMetric](docs/config.md#xmetric--指标) |
| `xgorm` | `*gorm.DB`，内置 MySQL / PostgreSQL | `xgorm.CWithCtx(ctx)`、`xgorm.C("name")` | [XGorm](docs/config.md#xgorm--数据库) |
| `xgorm/clickhouse` | 给 xgorm 加 ClickHouse 驱动 | 匿名 import，配置里 `Driver: clickhouse` | [其它驱动](docs/config.md#其它驱动) |
| `xredis` | `*redis.Client` | `xredis.C().Get(ctx, k)`、`xredis.C("name")` | [XRedis](docs/config.md#xredis) |
| `xcache` | 本地缓存（ristretto） | `xcache.Get` / `xcache.Set`，原生实例 `xcache.C()` | [XCache](docs/config.md#xcache--本地缓存) |
| `xhttp` | 出站 HTTP（`*resty.Client`），带链路与指标 | `xhttp.R(ctx).Get(url)`、`xhttp.C()`、`xhttp.RawClient()` | [XHttp](docs/config.md#xhttp--出站-http) |
| `xgin` | Gin Web 服务，内置日志、链路、指标、panic 恢复中间件 | `xone.MustRun(xgin.New().WithRoutes(...))` | [XGin](docs/config.md#xgin--web-服务) |
| `xginswagger` | Swagger UI | 在路由里 `xginswagger.Register(e, docs.SwaggerInfo)` | [XGinSwagger](docs/config.md#xginswagger) |

- 多实例的模块（`xgorm` / `xredis` / `xcache`）按名字取：`C()` 是 `default`，`C("orders")` 是配置里 `Clients.orders`。
- 在 `xone.Run` 把它建起来之前调 `C()` 会 panic，并说清是调早了、没配、还是名字写错。
  `xhttp` 例外，任何时候都有一个可用的客户端。
- 每个集成都导出纯构造器 `New`，测试里或需要额外实例时可以绕开框架直接构造。

## 写业务代码

### 在启动前 / 停止前做事：`xhook`

```go
func init() {
	xhook.BeforeStart(warmup) // 不写档位就是 StageBusiness：所有客户端已就绪，服务还没接流量
	xhook.BeforeStop(flush)   // 只在上面那个 warmup 成功之后才会被调用
}

func warmup(ctx context.Context) error {
	return xgorm.CWithCtx(ctx).Find(&hotItems).Error
}
```

钩子里写普通 Go 代码。`ctx` 在收到退出信号时取消，会阻塞的调用要把它传下去。
只有必须早于或晚于别人时才用 `xhook.At(xhook.StageClient)` 之类指定档位，
见 [架构：档位](docs/architecture.md#档位只在你必须早于或晚于别人时才写)。

### 服务本体：`Runnable`

交给 `xone.Run` 的只要实现一个方法：

```go
type Runnable interface {
	Start(context.Context) error
}
```

`Start` 在所有启动钩子之后调用，`ctx` 取消时返回即可；干完活 `return nil` 就是一次性任务。
光靠 `ctx` 停不下来的（比如要调 `Shutdown`）再加一个 `Stop(ctx context.Context) error`，可选。
`xgin.New()` 就是一个 Runnable；非 Web 服务自己写一个：

```go
type Consumer struct{ /* ... */ }

func (c *Consumer) Start(ctx context.Context) error {
	for {
		select {
		case <-ctx.Done():
			return nil // handle 是同步的，走到这里没有在途消息
		case m := <-c.messages:
			c.handle(context.WithoutCancel(ctx), m) // 处理中的消息不跟着退出信号一起被取消
		}
	}
}

func main() { xone.MustRun(&Consumer{}) }
```

消费者有三个容易踩的地方（等在途消息、`WithoutCancel`、别提前 `Close`），
见 [架构：consumer / job](docs/architecture.md#不是-web-服务怎么办consumer--job)
和 [`example/consumer/`](example/consumer/)。

### 读自己的配置：`xconfig.Unmarshal`

```go
type Config struct {
	Topic   string `yaml:"Topic"`
	Workers int    `yaml:"Workers"`
}

func main() {
	c := Config{Topic: "orders", Workers: 4} // 默认值预填，文件里没写的保持不变
	if err := xconfig.Unmarshal("MyApp", &c); err != nil {
		log.Fatal(err)
	}
	xone.MustRun(&Consumer{workers: c.Workers})
}
```

```yaml
MyApp:
  Topic: orders
  Workers: 8
```

在 `Start` 之前任何时候调都行，读到的永远是最终值。配置文件里出现没人读的顶层 key
会启动失败，所以业务自己的配置块一定要有人在 `main` 或 `BeforeStart` 钩子里 `Unmarshal`。详见 [业务自己的配置块](docs/config.md#业务自己的配置块)。

## 配置要点

- **文件位置**：`--config=<path>` > 环境变量 `XONE_CONFIG` > 约定路径
  `conf/application.yml`、`config/application.yml`、`application.yml`（`.yaml` 也认）。
  显式指定的找不到是错误；约定路径都没有则告警并全用默认值。
- **默认值预填**在结构体里，只写你要改的字段。时间写 `30s` / `1500ms`。
- **拼错字段、没人认领的块都是启动失败**，不是静默忽略。
- **环境变量**：`${VAR}` 必填，未设置则启动失败；`${VAR:default}` 可选。
- **Profile 与 Import**，写法同 Spring：`--profile=prod`（或 `XONE_PROFILE`）叠加
  `application-prod.yml`；`Import: [conf/shared.yml, optional:conf/local.yml]` 引入片段。
  map 递归合并、列表整体替换、标量覆盖。
- 编辑器补全：挂上仓库根的 `config_schema.json`，见 [编辑器补全](docs/config.md#编辑器补全)。

全部配置项和规则：[`docs/config.md`](docs/config.md)。

## 优雅退出

- 收到 SIGINT / SIGTERM 后取消 `Start` 的 `ctx`，调可选的 `Stop`，等 `Start` 返回，再逆序执行停止钩子。
- 整个退出流程**只有一个总预算** `xone.WithStopTimeout`（默认 15s），其中服务最多用 2/3（默认 10s），
  其余留给各组件关闭。部署环境的终止宽限期留得比它长即可。
- 启动期间收到信号：不再启动服务，已建好的逆序关掉，以 0 退出。
- 卡住时**再发一次信号**进程立即终止。

```go
xone.MustRun(app, xone.WithStopTimeout(30*time.Second))
```

细节见 [架构：退出信号与停止预算](docs/architecture.md#退出信号从进程起步的第一毫秒就接管)。

## 接下来

- [`docs/config.md`](docs/config.md)：全部配置项、合并规则、写自己的集成
- [`docs/architecture.md`](docs/architecture.md)：设计原则、档位与钩子配对、退出流程、与底层库默认值的差异
- [`docs/development.md`](docs/development.md)：给贡献者：仓库结构、脚本、测试与发布
- [`example/`](example/)：可直接运行的 Web 服务、消费者、自定义集成
