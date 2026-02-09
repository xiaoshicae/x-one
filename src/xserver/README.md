# XServer - 服务启动模块

💡 提供 `AxumServer` (HTTP) 和 `BlockingServer` (Consumer/Job) 的封装，统一处理信号监听、优雅停机和生命周期管理。

## 核心组件

### 1. AxumServer

适用于 Web 服务，集成了 `axum` 框架。


- **配置**: 通过 `XAxum` 配置端口和 Host。
- **特性**: 自动注入 Trace 中间件（待实现）、优雅停机。

```rust
use x_one::run_axum;
use axum::{Router, routing::get};

let app = Router::new().route("/", get(|| async { "Hello" }));
run_axum(app).await?;
```

### 2. BlockingServer

适用于后台任务、消息队列消费者等无需监听端口的服务。

- **特性**: 启动后阻塞主线程，直到收到 `SIGINT` / `SIGTERM` 信号。

```rust
use x_one::{run_server, BlockingServer};

// 在其他线程启动 Consumer
tokio::spawn(async {
    // consume_loop().await;
});

// 以 BlockingServer 阻塞等待退出信号
let server = BlockingServer::new();
run_server(&server).await?;
```

## 优雅停机流程

1. 收到 `SIGINT` (Ctrl+C) 或 `SIGTERM` 信号。
2. 打印 "Stop server begin"。
3. 调用 `server.stop()` 停止接收新请求（对于 HTTP 服务）。
4. 执行 `xhook::before_stop` 注册的所有钩子（如关闭数据库连接、刷新日志）。
5. 进程退出。
