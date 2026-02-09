# XTrace - 链路追踪模块

💡 基于 [OpenTelemetry](https://github.com/open-telemetry/opentelemetry-rust) 封装，提供分布式链路追踪能力。

## 功能特性

- **自动初始化**：根据配置自动初始化 TracerProvider。
- **导出器**：支持 Console 导出（调试用），未来可扩展 OTLP 等。
- **生命周期**：集成到框架的 `before_stop` 钩子，确保 Trace 数据在停机前发送完毕。

## 配置参数

```yaml
XTrace:
  Enable: true    # 是否开启 Trace
  Console: false  # 是否打印到控制台 (调试模式)
```

## 使用 Demo

```rust
use x_one::xtrace;
use opentelemetry::global;
use opentelemetry::trace::{Tracer, TraceContextExt};

// 检查 trace 是否启用
if xtrace::is_trace_enabled() {
    let tracer = global::tracer("my-lib");

    // 创建 Span
    tracer.in_span("operation_name", |cx| {
        // 业务逻辑
        // ...
    });
}
```

## 自动集成

`xlog` 已集成 Trace，会自动从 OpenTelemetry Context 中提取 `trace_id` 和 `span_id` 记录到日志中。当存在活跃的 OTel Span（如通过 `tracer.in_span(...)` 创建）时，JSON 和控制台日志会自动包含 `trace_id` 和 `span_id` 字段。