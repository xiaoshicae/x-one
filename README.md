# X-One

开箱即用的 Rust 三方库集成框架 SDK

## 功能特性

- **统一集成**：集成常用三方库（Axum、Sqlx、Moka、Reqwest、OpenTelemetry），降低维护成本
- **配置驱动**：通过 YAML 配置启用能力，开箱即用，支持多环境 Profile
- **最佳实践**：提供生产级默认参数配置（连接池、超时、日志轮转等）
- **生命周期**：支持 Hook 机制（BeforeStart / BeforeStop），灵活扩展
- **可观测性**：结构化 JSON 日志 + 全链路 Trace 集成（自动注入 trace_id / span_id）

## 环境要求

- Rust edition 2024（rustc >= 1.85）

## 快速开始

### 1. 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
x-one = { path = "." }  # 或 git 依赖
tokio = { version = "1", features = ["full"] }
axum = "0.8"
serde = { version = "1", features = ["derive"] }
```

### 2. 配置文件

创建 `application.yml`（支持放置在 `./`、`./conf/`、`./config/` 目录下）：

```yaml
Server:
  Name: "my-service"
  Version: "v1.0.0"
  Profiles:
    Active: "dev"

XAxum:
  Port: 8000

XLog:
  Level: "info"
  Console: true

XOrm:
  Driver: "mysql"
  DSN: "mysql://user:password@127.0.0.1:3306/dbname"

XHttp:
  Timeout: "30s"
  RetryCount: 3

XCache:
  MaxCapacity: 100000
  DefaultTTL: "5m"
```

### 3. 启动服务

```rust
use axum::{Router, routing::get};
use x_one::XAxum;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = XAxum::new()
        .with_route_register(|r| r.route("/ping", get(|| async { "pong" })))
        .build();

    // 自动初始化所有模块 + 启动 HTTP 服务 + 优雅停机
    x_one::run_server(&server).await?;
    Ok(())
}
```

### 4. 使用模块

```rust
use x_one::{xconfig, xcache, xhttp};
use x_one::{xlog_info, xlog_error};
use std::time::Duration;

async fn handler() {
    // 读取配置
    let port = xconfig::get_int("XAxum.Port");

    // HTTP 请求（reqwest 封装）
    let resp = xhttp::get("https://httpbin.org/get").send().await.unwrap();

    // 本地缓存（moka 封装，支持 per-entry TTL）
    xcache::set("key", "value".to_string());
    xcache::set_with_ttl("temp", 42, Duration::from_secs(60));
    let val: Option<String> = xcache::get("key");

    // 结构化日志（自动注入 trace_id）
    xlog_info!(user_id = 123, action = "login", "request handled");
}
```

## 模块清单

| 模块 | 底层库 | 说明 | 文档 |
|---|---|---|---|
| [xconfig](./src/xconfig/README.md) | serde_yaml | YAML 配置 / 环境变量 / Profile | 配置管理 |
| [xlog](./src/xlog/README.md) | tracing | 结构化 JSON 日志 / 文件轮转 / KV 注入 | 日志 |
| [xtrace](./src/xtrace/README.md) | opentelemetry | 分布式链路追踪 | Trace |
| [xhttp](./src/xhttp/README.md) | reqwest | HTTP 客户端 / 重试 / 连接池 | HTTP |
| [xorm](./src/xorm/README.md) | sqlx | MySQL / PostgreSQL 连接池管理 | 数据库 |
| [xcache](./src/xcache/README.md) | moka | 高性能本地缓存 / TTL / TinyLFU | 缓存 |
| [xhook](./src/xhook/README.md) | - | 生命周期钩子 / 排序执行 / 超时 | Hook |
| [xserver](./src/xserver/README.md) | axum | 服务启动 / 优雅停机 / 中间件 | 服务 |
| [xutil](./src/xutil/README.md) | humantime / backon | 工具函数库 | 工具 |

## 服务启动方式

```rust
use x_one::xhook::HookOptions;
use x_one::XAxum;
use axum::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注册自定义启动钩子（order >= 100 避免与内置模块冲突）
    x_one::before_start!(|| {
        println!("Custom initialization...");
        Ok(())
    }, HookOptions::new().order(100));

    // 注册自定义停止钩子
    x_one::before_stop!(|| {
        println!("Cleaning up resources...");
        Ok(())
    }, HookOptions::new().order(50));

    // 方式一：XAxum Builder 构建 Web 服务
    let server = XAxum::new()
        .with_route_register(|r| r.route("/", get(|| async { "Hello" })))
        .build();
    x_one::run_server(&server).await?;

    // 方式二：关闭内置中间件
    // let server = XAxum::new()
    //     .enable_log_middleware(false)
    //     .build();
    // x_one::run_server(&server).await?;

    // 方式三：阻塞服务（适用于 Consumer / Job）
    // x_one::run_blocking_server().await?;

    Ok(())
}
```

## 生命周期 API

除了通过 `run_axum` / `run_server` 自动管理，也可手动控制：

```rust
// 服务启动前准备工作(触发xhook管理的before hook，初始化资源)
// 触发 xconfig、xlog、xorm 等模块初始化
x_one::init().await?;

// 你自己的处理逻辑(服务启动或一次性脚本等)
// ...

// 服务退出前清理(触发xhook管理的after hook，释放资源等)
x_one::shutdown().ok();
```

## 环境变量

| 环境变量 | 说明 | 示例 |
|---|---|---|
| `SERVER_ENABLE_DEBUG` | 启用框架内部调试日志 | `true` |
| `SERVER_PROFILES_ACTIVE` | 指定激活的配置环境 | `dev`, `prod` |
| `SERVER_CONFIG_LOCATION` | 指定配置文件路径 | `/app/config.yml` |

配置文件支持环境变量占位符：

```yaml
XOrm:
  DSN: "${DB_DSN:-mysql://user:pass@localhost:3306/db}"
```

## 更新日志

- **v0.2.0** (2026-02-10) - xaxum 支持 h2c (HTTP/2 cleartext)、日志增加 caller/threadName 字段、HTTP 请求耗时人性化显示、幂等 Hook 注册
- **v0.1.0** (2026-02-10) - 初始版本，11 个模块（xconfig, xlog, xtrace, xhttp, xorm, xcache, xaxum, xhook, xserver, xutil, error）
