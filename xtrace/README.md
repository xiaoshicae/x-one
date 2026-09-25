# xtrace —— 链路

装好 OpenTelemetry 的全局 TracerProvider 和 Propagator，业务代码用原生的 OpenTelemetry API。
xgin / xgorm / xredis / xhttp 自带它，用了其中任何一个就不用另外 import。

## 快速上手

```go
// main.go
package main

import (
	"context"
	"log"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
	sdktrace "go.opentelemetry.io/otel/sdk/trace"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xgin"
	"github.com/xiaoshicae/x-one/xhttp"
	"github.com/xiaoshicae/x-one/xtrace"
)

func main() {
	// 框架不内置上报：要上报就自己挂一个 exporter，在 xone.Run 之前
	exp, err := otlptracegrpc.New(context.Background(),
		otlptracegrpc.WithEndpoint(os.Getenv("OTLP_ENDPOINT")), otlptracegrpc.WithInsecure())
	if err != nil {
		log.Fatal(err)
	}
	xtrace.AddSpanProcessor(sdktrace.NewBatchSpanProcessor(exp)) // 退出时由框架 Shutdown

	xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
		e.POST("/api/v1/orders/:id/pay", pay)
	}))
}

func pay(c *gin.Context) {
	// 入站 Span 由 xgin 开好了；要再细分一段，就用原生的 OpenTelemetry API
	ctx, span := otel.Tracer("order").Start(c.Request.Context(), "risk_check")
	span.SetAttributes(attribute.String("order.id", c.Param("id")))
	resp, err := xhttp.R(ctx).Get("http://risk.internal/api/v1/check") // traceparent 自动带给下游
	span.End()

	if err != nil || resp.IsError() {
		c.JSON(http.StatusBadGateway, gin.H{"error": "risk check unavailable"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "paid"})
}
```

```yaml
# conf/application.yml
XApp:
  Name: shop.order.api     # 就是 service.name
XTrace:
  SampleRatio: 0.1         # 只管根 Span；有上游时听上游的
  ForwardHeaders: [X-Request-Id]
XGin:
  TrustedProxies: ["10.0.0.0/8"]   # 透传 Header 和 baggage 只收这些对端发来的
```

## 重点

- **框架不内置任何 exporter**：不 `AddSpanProcessor` 的话 Span 照样生成、`trace_id` 照样进日志，只是不上报。本地调试开 `Console: true`。
- **透传 Header 和 `baggage` 只收可信对端的**：直连对端在 `XGin.TrustedProxies` 里才收。`TrustedProxies` 默认只信私有网段
  （负载均衡、K8s 的 Ingress 和 Pod），公网直连的不收。`traceparent` / `b3` 谁发来的都接。见
  [observability.md「传播与信任边界」](../docs/observability.md#传播与信任边界)。
- **有上游时一律听上游的 sampled 位**，`SampleRatio: 1` 也不例外。`SampleRatio: 0` 是不采样但照常生成、透传 TraceID；
  连 Span 都不要用 `Enable: false`。
- **`service.name`** 取 `XApp.Name`，环境变量 `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME` 压过它。见[「行为与实测」](#行为与实测)。
- `ForwardHeaderRules` 的 `Domains` 只认 `api.internal.com` 和 `*.trusted.com` 两种写法，`*.trusted.com` 不匹配裸域。

## 配置

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
  默认只信私有网段，公网直连的不收。`traceparent` / `b3` 谁发来的都接。细节见
  [observability.md「传播与信任边界」](../docs/observability.md#传播与信任边界)。
- `XGin.Trace` / `XHttp.Trace` 只管开不开 Span，关掉之后透传照常。
- `SampleRatio: 0` 是「不采样但照常生成、透传 TraceID」；要连 Span 都不产生用 `Enable: false`。
- `Domains` 只认 `api.internal.com` 和 `*.trusted.com` 两种写法；`*.trusted.com` 匹配任意层级子域、**不匹配裸域**。
  其余带 `*` 的写法、同一个 header 同时出现在 `ForwardHeaders` 和 `ForwardHeaderRules` 里，都在读配置时失败。
- `AddSpanProcessor` 登记的处理器在 `Enable: false` 时收不到 Span，退出时照样被 Shutdown。

## 行为与实测

实测环境和跨模块的总表见 [`docs/behavior.md`](../docs/behavior.md)。

OTel SDK v1.46.0、otelhttp v0.71.0。

**采样**：SDK 的 `AlwaysSample` 无视上游的 `sampled=00`，还把 `-01` 往下游传。这里用
`ParentBased(TraceIDRatioBased(SampleRatio))`：有上游时一律听上游的 sampled 位，`SampleRatio: 1` 也不例外。

**`service.name` 的优先级**，后面的压过前面的：OTel 自己的兜底名 `unknown_service:<可执行文件名>` →
`XApp.Name` / `XApp.Version` → `OTEL_RESOURCE_ATTRIBUTES` → `OTEL_SERVICE_NAME`。没配的那一项不写，
不会写进一个空的 `service.name=""` 把兜底名盖掉。

**resource 采集出错时只打一条告警、用采到的那部分继续**：`OTEL_RESOURCE_ATTRIBUTES` 写错一项、
或容器里以随机 UID 运行查不到当前用户，都不让服务起不来。

**透传只收可信对端的值。** OTel 自带的 `propagation.Baggage` 谁发来的都收：公网客户端发一个 `X-Tenant-Id`，
或者改写成 `baggage: tenant=…`，就被当成自己人给的、带进内网的每一次调用。规则见
[`observability.md`](../docs/observability.md#传播与信任边界)。

**`*trusted.com` 这类通配**：原样照字面后缀匹配的话 `*trusted.com` 会匹配 `eviltrusted.com`，
一个谁都能注册的域名就拿到了内部令牌。所以 `Domains` 只认 `api.internal.com` 和 `*.trusted.com` 两种写法。

## 可观测

### 日志

| 消息 | 级别 | 字段 |
|---|---|---|
| `xtrace ignored forward headers from an untrusted peer, …` | WARN | 整个进程只打一次 |

日志的全局约定（`trace_id` 注入、`xlog.AddKV`、框架的启停日志）见 [`docs/observability.md`](../docs/observability.md#日志)。
