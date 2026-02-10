# XAxum - HTTP 服务器模块

基于 [Axum](https://github.com/tokio-rs/axum) 封装，提供 Builder 模式构建 HTTP 服务器，内置日志和链路追踪中间件。

## 功能特性

- **Builder 模式**：通过 `XAxum` 链式 API 构建服务器，灵活配置
- **内置中间件**：日志中间件（访问日志 + 敏感 header 脱敏）、Trace 中间件（自动注入 OpenTelemetry 上下文）
- **h2c 支持**：可选启用 HTTP/2 明文（h2c），同时兼容 HTTP/1.1 自动检测
- **配置驱动**：监听地址从 YAML 配置自动读取，也可手动覆盖
- **优雅停机**：实现 `Server` trait，配合 `run_server` 自动处理信号和资源清理
- **启动 Banner**：服务启动时打印 ASCII art 标识

## 配置

```yaml
XAxum:
  Host: "0.0.0.0"       # 监听地址（默认 0.0.0.0）
  Port: 8000             # 端口号（默认 8000）
  UseHttp2: false        # 是否启用 HTTP/2
  EnableBanner: true     # 是否打印启动 Banner（默认 true）
```

## 使用

### 基本用法

```rust
use axum::routing::get;
use x_one::XAxum;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = XAxum::new()
        .with_route_register(|r| r.route("/ping", get(|| async { "pong" })))
        .build();

    x_one::run_server(&server).await?;
    Ok(())
}
```

### 从已有 Router 构建

```rust
use axum::{Router, routing::get};
use x_one::XAxum;

let app = Router::new().route("/ping", get(|| async { "pong" }));
let server = XAxum::from_router(app).build();
```

### 多路由注册

```rust
use axum::routing::{get, post};
use x_one::XAxum;

let server = XAxum::new()
    .with_route_register(|r| r.route("/health", get(|| async { "ok" })))
    .with_route_register(|r| r.route("/api/data", post(|| async { "created" })))
    .build();
```

### 自定义中间件

```rust
use x_one::XAxum;

let server = XAxum::new()
    .with_middleware(|router| {
        // 添加自定义中间件层
        router
    })
    .build();
```

### 控制内置中间件

```rust
use x_one::XAxum;

let server = XAxum::new()
    .enable_log_middleware(false)      // 关闭日志中间件
    .enable_trace_middleware(false)    // 关闭追踪中间件
    .build();
```

### 启用 h2c（HTTP/2 明文）

```rust
use x_one::XAxum;

let server = XAxum::new()
    .use_http2(true)
    .build();
```

启用后，服务器同时支持 HTTP/1.1 和 HTTP/2 明文连接（h2c 自动检测），无需 TLS。

### 手动指定地址

```rust
use x_one::XAxum;

let server = XAxum::new()
    .addr("127.0.0.1:3000")
    .build();
```

## Builder API

| 方法 | 说明 | 默认值 |
|---|---|---|
| `XAxum::new()` | 创建默认构建器 | - |
| `XAxum::from_router(router)` | 从已有 Router 创建 | - |
| `.addr("host:port")` | 设置监听地址 | 从配置读取，或 `0.0.0.0:8000` |
| `.use_http2(bool)` | 开关 h2c（HTTP/2 明文） | `false` |
| `.enable_banner(bool)` | 开关启动 Banner | `true` |
| `.enable_log_middleware(bool)` | 开关日志中间件 | `true` |
| `.enable_trace_middleware(bool)` | 开关追踪中间件 | `true` |
| `.with_route_register(fn)` | 注册路由回调（可多次） | - |
| `.with_middleware(fn)` | 注入自定义中间件（可多次） | - |
| `.build()` | 构建 `XAxumServer` | - |

## 内置中间件

### 日志中间件（log_middleware）

- 记录请求/响应关键信息：method、path、status、headers、body、耗时
- 敏感 header 自动脱敏（`Authorization`、`Cookie` 等）
- 二进制/大体积 body 自动跳过

### 追踪中间件（trace_middleware）

- 从入站 HTTP header 提取 W3C `traceparent` 上下文
- 自动创建 OpenTelemetry Span，注入 `http.method`、`http.route` 等属性
- 下游 handler 中的 xlog 日志自动携带 `trace_id` / `span_id`

## 地址解析优先级

1. Builder `.addr()` 手动指定
2. YAML 配置 `XAxum.Host` + `XAxum.Port`
3. 默认值 `0.0.0.0:8000`
