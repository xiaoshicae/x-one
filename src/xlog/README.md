# XLog - 日志模块

基于 [tracing](https://github.com/tokio-rs/tracing) + tracing-subscriber 封装，提供结构化 JSON 日志、文件轮转、控制台输出、异步写入。

## 功能特性

- **结构化日志**：默认 JSON 格式，便于 ELK / Loki 收集
- **文件轮转**：通过 tracing-appender 按天切割日志文件
- **异步写入**：tracing-appender non_blocking 实现非阻塞写入
- **Trace 集成**：自动从 OpenTelemetry Context 注入 `trace_id` / `span_id`
- **KV 注入**：通过 `xlog_kv!` 宏向 Span 作用域内的日志自动注入自定义字段

## 配置参数

```yaml
XLog:
  Level: "info"             # 日志级别: trace / debug / info / warn / error
  Name: "app"               # 日志文件名前缀（生成 app.log）
  Path: "./log"             # 日志输出目录
  Console: true             # 是否同时输出到控制台
  ConsoleFormatIsRaw: false # 控制台是否输出原始 JSON（默认 false，输出带颜色文本）
```

## 使用

### 基础日志

```rust
use x_one::{xlog_info, xlog_error, xlog_warn, xlog_debug};

xlog_info!("Server started at port {}", 8080);
xlog_error!(error = "connection refused", "Database connection failed");
xlog_warn!(retries = 3, "Request retry exhausted");
```

### KV 字段注入

通过 `xlog_kv!` 创建 guard，guard 存活期间当前作用域内所有日志自动携带指定字段，无需逐个传参。

```rust
use x_one::{xlog_kv, xlog_info};

async fn handle_order(user_id: &str, order_id: &str) {
    let _guard = xlog_kv!(user_id = %user_id, order_id = %order_id);

    xlog_info!("received order");
    // {"msg":"received order", "user_id":"u123", "order_id":"o456", ...}

    validate(order_id);
    // validate 内部日志也自动携带 user_id、order_id

    xlog_info!("order done");
    // guard drop 后，后续日志不再携带
}

fn validate(order_id: &str) {
    xlog_info!("validating");
    // {"msg":"validating", "user_id":"u123", "order_id":"o456", ...}
}
```

**注意**：变量名必须用 `_guard`（或任意 `_xxx`），不能写 `_`——`_` 会立即 drop，KV 不生效。

### 结构化字段

```rust
use x_one::xlog_info;

xlog_info!(
    user_id = 123,
    action = "login",
    ip = "192.168.1.1",
    "User login success"
);
```

## JSON 日志字段

生成的 JSON 日志包含以下字段：

| 字段 | 说明 |
|---|---|
| `timestamp` | ISO 8601 时间戳 |
| `level` | 日志级别 |
| `target` | 模块路径 |
| `msg` | 消息内容 |
| `trace_id` | OpenTelemetry Trace ID（存在活跃 Span 时） |
| `span_id` | OpenTelemetry Span ID（存在活跃 Span 时） |
| 自定义字段 | 通过 `xlog_kv!` 或日志宏注入的 KV 字段 |
