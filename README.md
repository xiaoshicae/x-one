# xone

Go 三方库集成框架。import 想用的集成包，写一个配置文件，调一次 `xone.Run`：框架统一读配置、按阶段初始化各组件、
收到退出信号后逆序关闭。业务代码拿到的是**原生 client**——`*gorm.DB`、`*redis.Client`、`*gin.Engine`、标准库 `slog`——
不用学一套包装过的 API，也没有装配样板。

- **配置驱动**：行为写在 YAML 里，默认值预填，拼错字段直接启动失败
- **按阶段启动、逆序关闭**：日志 → 链路/指标 → 客户端 → 业务 → 服务，关闭反过来，只有一个总的停止预算
- **用什么付什么**：每个带三方依赖的集成是独立的 Go module，不用的不进你的模块图

## 安装

核心要 Go 1.22+，集成要 Go 1.25+。每个集成是**独立的 module**，要分别 `go get`：

```bash
go get github.com/xiaoshicae/x-one        # 核心：Run、xhook、xconfig、xlog、xflow、xapp、xerror、xtls、xonetest
go get github.com/xiaoshicae/x-one/xgin   # 按需：xtrace xmetric xgorm xgorm/clickhouse xredis xcache xhttp xgin xginswagger
```

> 各模块还没打 tag。在那之前把仓库克隆到本地，在你的 `go.mod` 里用 `replace` 指向它——
> 用到的集成和它们依赖的仓库内模块都要写（replace 不会从依赖的 `go.mod` 里传过来），多写的不影响：
>
> ```
> require (
> 	github.com/xiaoshicae/x-one v0.0.0
> 	github.com/xiaoshicae/x-one/xgin v0.0.0
> )
>
> replace (
> 	github.com/xiaoshicae/x-one => ../x-one
> 	github.com/xiaoshicae/x-one/xtrace => ../x-one/xtrace
> 	github.com/xiaoshicae/x-one/xmetric => ../x-one/xmetric
> 	github.com/xiaoshicae/x-one/xgin => ../x-one/xgin
> 	github.com/xiaoshicae/x-one/xgorm => ../x-one/xgorm
> 	github.com/xiaoshicae/x-one/xgorm/clickhouse => ../x-one/xgorm/clickhouse
> 	github.com/xiaoshicae/x-one/xredis => ../x-one/xredis
> 	github.com/xiaoshicae/x-one/xcache => ../x-one/xcache
> 	github.com/xiaoshicae/x-one/xhttp => ../x-one/xhttp
> 	github.com/xiaoshicae/x-one/xginswagger => ../x-one/xginswagger
> )
> ```
>
> 然后 `go mod tidy`。`xgorm` 内置 MySQL 和 PostgreSQL；ClickHouse 驱动是单独的 `xgorm/clickhouse`，用到才加。

## 快速开始

```go
// main.go
package main

import (
	"log/slog"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xgin" // 日志、链路、指标跟着 xgin 一起来，不用另外 import
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
# Ctrl+C 优雅退出，卡住时再按一次立即终止
```

更完整的可运行示例在 [`example/`](example/)（一个 module，进各自的目录跑）：

```bash
cd example && go run . --config=application.yml            # Web 服务：日志、链路、指标、缓存
cd example/consumer && go run . --config=application.yml   # 消息队列消费者（非 Web）
cd example/component && go run . --config=application.yml  # 自己写一个集成
```

## 模块一览

| 模块 | 给你什么 | 怎么用 | 配置 |
|---|---|---|---|
| `xlog`（核心） | 基于 `log/slog` 的日志，有链路时自动带 `trace_id` | `slog.InfoContext(ctx, ...)` | [XLog](xlog/README.md#配置) |
| `xapp`（核心） | 应用名、版本 | `xapp.Name()` / `xapp.Version()` | [App](xapp/README.md#配置) |
| `xflow`（核心） | 流程编排，失败自动回滚 | `xflow.New[T](name, steps...).Execute(ctx, data)` | [XFlow](xflow/README.md#配置) |
| `xconfig` / `xhook`（核心） | 读自己的配置、在启动前 / 停止前做事 | `xconfig.Unmarshal("MyApp", &c)`、`xhook.BeforeStart(fn)` | [指南](docs/guide.md#读自己的配置) |
| `xerror`（核心） | 带模块名和操作名的错误 | `xerror.Is(err, "xconfig")`、`xerror.Module(err)` | [指南](docs/guide.md#错误处理) |
| `xtls`（核心） | 客户端 TLS 块，XGorm / XRedis / XHttp 共用 | 配置里的 `TLS:` | [TLS 块](xtls/README.md#配置) |
| `xonetest`（核心） | 测试里换一份配置、跑一遍钩子 | `xonetest.UseConfigYAML(t, yml)`、`xonetest.StartHooks(t)` | [指南](docs/guide.md#测试) |
| `xtrace` | OpenTelemetry 链路，设为全局 TracerProvider；xgin / xgorm / xredis / xhttp 自带 | `otel.Tracer("app").Start(ctx, "op")` | [XTrace](xtrace/README.md#配置) |
| `xmetric` | Prometheus 指标 | `xmetric.CounterInc(...)`、`xmetric.Registry()` | [XMetric](xmetric/README.md#配置) |
| `xgorm` | `*gorm.DB`，内置 MySQL / PostgreSQL | `xgorm.CWithCtx(ctx)`、`xgorm.C("name")` | [XGorm](xgorm/README.md#配置) |
| `xgorm/clickhouse` | 给 xgorm 加 ClickHouse 驱动 | 匿名 import，配置里 `Driver: clickhouse` | [其它驱动](xgorm/clickhouse/README.md#配置) |
| `xredis` | `*redis.Client` | `xredis.C().Get(ctx, k)`、`xredis.C("name")` | [XRedis](xredis/README.md#配置) |
| `xcache` | 本地缓存（ristretto） | `xcache.Get` / `xcache.Set`，原生实例 `xcache.C()` | [XCache](xcache/README.md#配置) |
| `xhttp` | 出站 HTTP（`*resty.Client`），带链路与指标 | `xhttp.R(ctx).Get(url)` | [XHttp](xhttp/README.md#配置) |
| `xgin` | Gin Web 服务，内置访问日志、链路、指标、panic 恢复 | `xone.MustRun(xgin.New().WithRoutes(...))` | [XGin](xgin/README.md#配置) |
| `xginswagger` | Swagger UI | 在路由里 `xginswagger.Register(e, docs.SwaggerInfo)` | [XGinSwagger](xginswagger/README.md#配置) |

## 核心概念

- **钩子与档位**：`xhook.BeforeStart` / `BeforeStop` 登记，框架按五个档位依次执行，业务钩子不写档位——[钩子与档位](docs/guide.md#钩子与档位)
- **一对钩子**：停止钩子只在配对的启动钩子成功之后才执行，档位也跟着它——[停止钩子](docs/guide.md#停止钩子配对与继承)
- **Runnable**：交给 `xone.Run` 的服务只要写 `Start`，停不下来的再加 `Stop`——[Runnable](docs/guide.md#runnable)
- **实例与 `C()`**：`xgorm` / `xredis` / `xcache` 按名字取实例，调早了、没配、名字写错都说清楚——[多实例](docs/guide.md#多实例)
- **停止预算**：整个退出流程只有 `xone.WithStopTimeout` 一份（默认 15s），服务最多用 2/3——[部署](docs/guide.md#部署)

## 接下来

- [`docs/guide.md`](docs/guide.md)：使用指南——钩子、Runnable、读配置、非 Web 服务、测试、部署、写自己的集成
- 各模块目录下的 `README.md`（上面「模块一览」的配置一列链到它）：这个模块的全部配置字段、量出来的行为、日志 / 指标 / Span、排错
- [`docs/config.md`](docs/config.md)：配置参考——文件位置、Profile、Import、合并规则、占位符，以及配置块到模块文档的索引
- [`docs/behavior.md`](docs/behavior.md)：与底层库不同的默认值总表，以及跨模块的启动期建连探测
- [`docs/observability.md`](docs/observability.md)：日志、指标、链路的全局约定，链路传播与信任边界
- [`docs/troubleshooting.md`](docs/troubleshooting.md)：按错误原文排错（跨模块的那些）
- [`docs/architecture.md`](docs/architecture.md)：为什么是现在这个样子
- [`docs/development.md`](docs/development.md)：给贡献者——仓库结构、脚本、e2e、规矩
