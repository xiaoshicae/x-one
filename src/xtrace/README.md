# XTrace - 链路追踪模块

基于 [OpenTelemetry](https://github.com/open-telemetry/opentelemetry-rust) 封装，提供分布式链路追踪能力。

## 功能特性

- **自动初始化**：根据配置自动初始化 TracerProvider
- **导出器**：支持 Console 导出（调试用），可扩展 OTLP 等
- **日志集成**：xlog 自动从 OpenTelemetry Context 提取 `trace_id` / `span_id` 写入日志
- **生命周期**：集成 `before_stop` 钩子（order=1），确保 Trace 数据在停机前发送完毕

## 配置参数

```yaml
XTrace:
  Enable: true    # 是否开启 Trace
  Console: false  # 是否打印到控制台（调试模式）
```

## 使用

```rust
use x_one::xtrace;
use opentelemetry::global;
use opentelemetry::trace::Tracer;

if xtrace::is_trace_enabled() {
    let tracer = global::tracer("my-service");

    tracer.in_span("operation_name", |_cx| {
        // 业务逻辑
        // 此作用域内的 xlog 日志自动携带 trace_id / span_id
    });
}
```
