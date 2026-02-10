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

### Server Trait

所有服务必须实现 `Server` trait：

```rust
pub trait Server: Send + Sync {
    async fn run(&self) -> Result<(), XOneError>;
    async fn stop(&self) -> Result<(), XOneError>;
}
```

### 手动调用示例

```rust
// 不启动 HTTP 服务，仅使用框架能力
x_one::init().await?;

// 使用 xconfig、xlog、xorm 等模块...

x_one::shutdown().ok();
```

## 运行模式

### 1. XAxumServer（HTTP 服务）

适用于 Web 服务，集成了 axum 框架。

- 通过 `XAxum` Builder 构建服务器
- 通过 `XAxum` 配置端口和 Host
- 内置 log / trace 中间件（可通过 Builder 控制开关）
- 启动时打印 ASCII art Banner

```rust
use axum::routing::get;
use x_one::XAxum;

// 通过 Builder 构建（默认启用所有中间件）
let server = XAxum::new()
    .with_route_register(|r| r.route("/", get(|| async { "Hello" })))
    .build();
x_one::run_server(&server).await?;

// 关闭日志中间件
let server = XAxum::new()
    .enable_log_middleware(false)
    .build();
x_one::run_server(&server).await?;
```

### 2. BlockingServer（后台服务）

适用于消息队列消费者、定时任务等无需监听端口的服务。

```rust
// 在其他线程启动 Consumer
tokio::spawn(async {
    // consume_loop().await;
});

// 以 BlockingServer 阻塞等待退出信号
let server = x_one::BlockingServer::new();
x_one::run_server(&server).await?;
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

1. 收到 `SIGINT` (Ctrl+C) 或 `SIGTERM` 信号
2. 调用 `server.stop()` 停止接收新请求
3. 执行所有 `before_stop` 钩子（按 order 从小到大）：
   - xtrace（order=1）：刷新链路数据
   - xcache（order=2）：清理缓存实例
   - xorm（order=3）：关闭数据库连接池
   - xlog guard（order=100）：刷新并关闭日志写入器
4. 合并 server 结果和 hook 结果后返回
