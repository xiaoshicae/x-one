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
use opentelemetry::trace::{Tracer, TraceContextExt};

// 获取 Tracer
let tracer = xtrace::get_tracer("my-lib");

// 创建 Span
tracer.in_span("operation_name", |cx| {
    // 业务逻辑
    // ...
});
```

## 自动集成

目前 `xlog` 已集成 Trace，会自动从 Context 中提取 `trace_id` 和 `span_id` 记录到日志中（需配合 `tracing-opentelemetry` 使用）。