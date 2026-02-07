# XLog - 日志模块

💡 基于 [tracing](https://github.com/tokio-rs/tracing) 封装，提供结构化 JSON 日志、文件轮转、控制台输出、异步写入等功能。

## 功能特性

- **结构化日志**：默认 JSON 格式，便于 ELK/Loki 收集。
- **文件轮转**：支持按天切割日志文件。
- **异步写入**：使用 `tracing-appender` 实现非阻塞写入，不影响业务性能。
- **动态配置**：支持通过配置调整级别、输出目标等。

## 配置参数

```yaml
XLog:
  Level: "info"             # 日志级别: trace/debug/info/warn/error
  Name: "app"               # 日志文件名 (默认 app.log)
  Path: "./log"             # 日志路径
  Console: true             # 是否输出到控制台
  ConsoleFormatIsRaw: false # 控制台是否输出原始 JSON (默认 false，即输出带颜色的文本)
  RotateTime: "1d"          # 切割周期 (目前仅支持按天 rolling::daily)
```

## 使用 Demo

```rust
use x_one::xlog::{xlog_info, xlog_error, xlog_warn, xlog_debug};

fn main() {
    // 基础日志
    xlog_info!("Server started at port {}", 8080);

    // 结构化字段
    xlog_info!(
        user_id = 123,
        action = "login",
        "User login success"
    );

    // 错误日志
    let err = "connection refused";
    xlog_error!(error = ?err, "Database connection failed");
}
```

## 字段说明

生成的 JSON 日志包含以下核心字段：
- `timestamp`: 时间戳
- `level`: 日志级别
- `target`: 模块/位置
- `msg`: 消息内容
- `thread_id`: 线程 ID
- ...以及用户自定义的 KV 字段