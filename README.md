# X-One

开箱即用的 Rust 微服务框架 SDK

## 💡 功能特性

- **统一集成**：集成常用三方库（Auxm〔基于 axum〕, Sqlx, Moka, Reqwest, OpenTelemetry），降低维护成本
- **配置驱动**：通过 YAML 配置启用能力，开箱即用，支持多环境 Profile
- **最佳实践**：提供生产级默认参数配置（连接池、超时、日志轮转等）
- **生命周期**：支持 Hook 机制（BeforeStart/BeforeStop），灵活扩展
- **可观测性**：全链路 Trace 集成（HTTP -> DB/Client）

## 🛠 环境要求

- Rust >= 1.75

## 🚀 快速开始

### 1. 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
x-one = { path = "." } # 或 git 依赖
tokio = { version = "1", features = ["full"] }
axum = "0.7"
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
  Auxm: # 对应 Auxm HTTP 服务（基于 axum）
    Port: 8000

XLog:
  Level: "info"
  Console: true

XOrm:
  Driver: "mysql"
  DSN: "user:password@tcp(127.0.0.1:3306)/dbname"

XHttp:
  Timeout: "30s"
  RetryCount: 3

XCache:
  MaxCapacity: 100000
  DefaultTTL: "5m"
```

### 3. 启动服务

```rust
use x_one::xserver::auxm::AuxmServer;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化配置和模块
    x_one::init_all();

    // 2. 构建路由
    let app = Router::new().route("/ping", get(|| async { "pong" }));

    // 3. 启动服务（自动处理平滑关闭）
    x_one::run_auxm(app).await?;

    Ok(())
}
```

### 4. 使用模块

```rust
use x_one::{xorm, xhttp, xlog, xcache, xconfig};
use x_one::xlog::xlog_info;

async fn handler() {
    // 数据库操作 (sqlx)
    // let pool = xorm::get_pool("default"); 
    
    // HTTP 请求 (reqwest)
    // let client = xhttp::client();
    // let resp = client.get("https://api.example.com").send().await?;

    // 本地缓存 (moka)
    xcache::set_with_ttl("key", "value", std::time::Duration::from_secs(60));

    // 日志记录
    xlog_info!("request handled");

    // 读取配置
    let port = xconfig::get_int("Server.Auxm.Port");
}
```

## 🧩 模块清单

| 模块 | 底层库 | 文档 | Log | Trace | 说明 |
|---|---|---|---|---|---|
| [xconfig](./src/xconfig/README.md) | [serde_yaml](https://github.com/dtolnay/serde-yaml) | 配置管理 | - | - | YAML配置/环境变量/Profile |
| [xlog](./src/xlog/README.md) | [tracing](https://github.com/tokio-rs/tracing) | 日志记录 | - | ✅ | 结构化日志/文件轮转 |
| [xtrace](./src/xtrace/README.md) | [opentelemetry](https://github.com/open-telemetry/opentelemetry-rust) | 链路追踪 | - | - | 分布式链路追踪 |
| [xorm](./src/xorm/README.md) | [sqlx](https://github.com/launchbadge/sqlx) | 数据库 | ✅ | ✅ | MySQL/PostgreSQL 连接池 |
| [xhttp](./src/xhttp/README.md) | [reqwest](https://github.com/seanmonstar/reqwest) | HTTP客户端 | - | - | 支持重试/连接池配置 |
| [xcache](./src/xcache/README.md) | [moka](https://github.com/moka-rs/moka) | 本地缓存 | - | - | 高性能本地缓存(TTL/LFU) |
| [xserver](./src/xserver/README.md) | [axum](https://github.com/tokio-rs/axum) | HTTP服务 | ✅ | ✅ | Web服务启动封装 |

## 🏗 服务启动方式

```rust
use x_one::xhook;

fn init_hooks() {
    // 注册启动前钩子
    xhook::before_start("custom_init", || {
        println!("Custom initialization...");
        Ok(())
    }, Default::default());

    // 注册停止前钩子
    xhook::before_stop("custom_cleanup", || {
        println!("Cleaning up resources...");
        Ok(())
    }, Default::default());
}

#[tokio::main]
async fn main() -> x_one::Result<()> {
    init_hooks();
    
    // 方式一：Auxm Web 服务
    // x_one::run_auxm(app).await
    
    // 方式二：Auxm HTTPS 服务
    // x_one::run_auxm_tls(app, "cert.pem", "key.pem").await

    // 方式三：阻塞服务（适用于 Consumer/Job）
    x_one::run_blocking_server().await
}
```

## 🔧 环境变量

| 环境变量 | 说明 | 示例 |
|---|---|---|
| `SERVER_ENABLE_DEBUG` | 启用 XOne 内部调试日志 | `true` |
| `SERVER_PROFILES_ACTIVE` | 指定激活的配置环境 | `dev`, `prod` |
| `SERVER_CONFIG_LOCATION` | 指定配置文件路径 | `/app/config.yml` |

配置文件支持环境变量占位符：

```yaml
XOrm:
  DSN: "${DB_DSN:-mysql://user:pass@localhost:3306/db}"
```

## 📝 更新日志

- **v0.1.0** (2026-02-07) - 初始版本移植自 Go xone 框架
- **v0.1.1** (2026-02-07) - Auxm 命名统一（破坏性变更，见 `MIGRATION.md`）
