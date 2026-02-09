# XServer - 服务启动与生命周期管理

提供对称的 `init()` / `shutdown()` 生命周期 API，以及 `AxumServer`（HTTP）和 `BlockingServer`（Consumer/Job）两种运行模式。

## 生命周期 API

```
init() ─── 注册内置 hook → 执行 before_start hooks ───┐
                                                       │
                          server.run()                 │  run_with_signal
                          信号监听 (SIGINT/SIGTERM)     │  内部对称包裹
                                                       │
shutdown() ── 执行 before_stop hooks（失败只 warn）────┘
```

- **`init()`**：幂等，注册内置模块 hook 并执行 `before_start` hooks。使用 `run_server` 时自动调用；不使用 server 时可手动调用。
- **`shutdown()`**：执行 `before_stop` hooks 清理资源。单个 hook 失败不中断后续执行。使用 `run_server` 时自动调用；不使用 server 时可手动调用。

### 手动调用示例

```rust
// 不启动 HTTP 服务，仅使用框架能力
x_one::init().expect("init failed");

// 使用 xconfig、xlog、xorm 等模块...

x_one::shutdown().ok();
```

## 运行模式

### 1. AxumServer（HTTP 服务）

```rust
use x_one::run_axum;
use axum::{Router, routing::get};

let app = Router::new().route("/", get(|| async { "Hello" }));
run_axum(app).await?;
```

### 2. BlockingServer（后台服务）

适用于消息队列消费者、定时任务等无需监听端口的服务。

```rust
use x_one::run_blocking_server;

// 在其他线程启动 Consumer
tokio::spawn(async {
    // consume_loop().await;
});

// 阻塞等待退出信号
run_blocking_server().await?;
```

### 3. 自定义 Server

实现 `Server` trait 即可接入生命周期管理。

```rust
use x_one::{Server, XOneError, run_server};

struct MyServer;

impl Server for MyServer {
    async fn run(&self) -> Result<(), XOneError> { /* ... */ Ok(()) }
    async fn stop(&self) -> Result<(), XOneError> { /* ... */ Ok(()) }
}

run_server(&MyServer).await?;
```

## 优雅停机流程

1. 收到 `SIGINT`（Ctrl+C）或 `SIGTERM` 信号
2. 调用 `server.stop()` 停止接收新请求
3. 执行所有 `before_stop` hooks（关闭数据库连接、刷新日志等）
4. 单个 hook 失败只 warn，保证所有清理逻辑都执行
5. 进程退出
