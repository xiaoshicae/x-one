# xtrace —— 链路

OpenTelemetry 链路，设为全局 TracerProvider；xgin / xgorm / xredis / xhttp 自带，用了其中任何一个就不用另外 import。
业务代码用原生的 OpenTelemetry API：

```go
ctx, span := otel.Tracer("app").Start(ctx, "op")
defer span.End()
```

链路的全貌、传播与信任边界见 [`docs/observability.md`「链路」](../docs/observability.md#链路)。

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
  默认一个都不信，于是**默认什么都不透传**。`traceparent` / `b3` 谁发来的都接。细节见
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
`App.Name` / `App.Version` → `OTEL_RESOURCE_ATTRIBUTES` → `OTEL_SERVICE_NAME`。没配的那一项不写，
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
