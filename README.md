# X-One

开箱即用的 Rust 三方库集成框架

## 功能特性

- 统一集成三方库（Axum、Sqlx、Moka、Reqwest、OpenTelemetry），降低维护成本
- 通过 YAML 配置启用能力，开箱即用
- 提供最佳实践的默认参数配置
- 支持 Hook 机制，灵活扩展生命周期
- 集成 OpenTelemetry 链路追踪，日志自动关联 TraceID

## 环境要求

- Rust edition 2024（rustc >= 1.85）

## 快速开始

### 1. 安装

```toml
[dependencies]
x-one = "0.2"
tokio = { version = "1", features = ["full"] }
axum = "0.8"
```

### 2. 创建配置文件

创建 `application.yml`（支持放置在 `./`、`./conf/`、`./config/` 目录下）：

```yaml
Server:
  Name: "my-service"
  Version: "v1.0.0"

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

### 3. 配置 Schema 校验（可选）

项目提供了 JSON Schema 文件，配置后 IDE 会自动补全字段并校验配置值。

**VS Code**（需安装 [YAML 扩展](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)）

在 YAML 文件首行添加：

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/xiaoshicae/x-one/main/config_schema.json
```

或在项目 `.vscode/settings.json` 中统一配置：

```json
{
  "yaml.schemas": {
    "https://raw.githubusercontent.com/xiaoshicae/x-one/main/config_schema.json": [
      "application.yml",
      "application-*.yml"
    ]
  }
}
```

**JetBrains（RustRover / IntelliJ）**

`Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings`，添加映射：

- Schema URL：`https://raw.githubusercontent.com/xiaoshicae/x-one/main/config_schema.json`
- Schema version：JSON Schema version 7
- 文件匹配：`application*.yml`

### 4. 启动服务

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

启动后框架会自动完成：配置加载 → 链路追踪初始化 → 日志初始化 → HTTP 客户端初始化 → 数据库连接 → 启动 Axum 服务。

## 模块清单

| 模块 | 底层库 | 说明 | Log | Trace |
|---|---|---|---|---|
| [xconfig](./src/xconfig/README.md) | serde_yaml | 配置管理（YAML + 环境变量 + Profile） | - | - |
| [xlog](./src/xlog/README.md) | tracing | 结构化日志（文件轮转 + KV 注入） | - | - |
| [xtrace](./src/xtrace/README.md) | opentelemetry | 链路追踪（W3C Trace Context） | - | - |
| [xhttp](./src/xhttp/README.md) | reqwest | HTTP 客户端（重试 + 连接池） | - | - |
| [xorm](./src/xorm/README.md) | sqlx | 数据库（MySQL / PostgreSQL，多数据源） | - | - |
| [xcache](./src/xcache/README.md) | moka | 本地缓存（支持 TTL / 泛型） | - | - |
| xserver | - | 服务运行和生命周期管理 | - | - |
| [xaxum](./src/xaxum/README.md) | axum | Axum Web 框架集成（Builder 模式 + 内置中间件） | - | - |

## 服务启动方式

```rust
use x_one::xhook::HookOptions;
use x_one::XAxum;
use axum::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 方式一：XAxum Builder 构建 Web 服务（推荐）
    let server = XAxum::new()
        .with_route_register(|r| r.route("/", get(|| async { "Hello" })))
        .build();
    x_one::run_server(&server).await?;

    // 方式二：通过 xserver::run_server 启动（等价于方式一）
    // x_one::run_server(&server).await?;

    // 方式三：自定义 Server（实现 Server trait）
    // x_one::run_server(&my_server).await?;

    // 方式四：阻塞服务（consumer / job 场景）
    // x_one::run_blocking_server().await?;

    // 方式五：仅初始化模块，不启动服务（调试用）
    // x_one::init().await?;

    Ok(())
}
```

## 使用模块

### 日志

```rust
use x_one::{xlog_info, xlog_error};

// 基础日志（自动注入 trace_id / span_id）
xlog_info!("user login success");

// 结构化 KV
xlog_info!(user_id = 123, action = "login", "request handled");
```

### HTTP 客户端

```rust
use x_one::xhttp;

// 发起 GET 请求
let resp = xhttp::get("https://api.example.com/users").send().await?;

// 发起 POST 请求
let resp = xhttp::post("https://api.example.com/users")
    .json(&body)
    .send()
    .await?;
```

配置示例：

```yaml
XHttp:
  Timeout: "30s"
  RetryCount: 3
  MaxIdleConnsPerHost: 100
```

### 数据库

```rust
use x_one::xorm;

// 获取连接池（默认实例）
let pool = xorm::db();

// 多数据源
let master = xorm::db_by_name("master");
let slave = xorm::db_by_name("slave");
```

单数据库配置：

```yaml
XOrm:
  Driver: "mysql"
  DSN: "mysql://user:pass@127.0.0.1:3306/mydb"
  MaxOpenConns: 50
  EnableLog: true
  SlowThreshold: "3s"
```

多数据库配置：

```yaml
XOrm:
  - Name: "master"
    Driver: "mysql"
    DSN: "mysql://user:pass@127.0.0.1:3306/master_db"
  - Name: "slave"
    Driver: "postgres"
    DSN: "postgres://postgres@127.0.0.1:5432/slave_db"
```

### 本地缓存

```rust
use x_one::xcache;
use std::time::Duration;

// 设置缓存
xcache::set("key", "value".to_string());

// 带 TTL
xcache::set_with_ttl("temp", 42, Duration::from_secs(60));

// 获取缓存（泛型）
let val: Option<String> = xcache::get("key");
```

### 自定义配置

```rust
use x_one::xconfig;

// 读取单个值
let port: Option<i64> = xconfig::get_int("XAxum.Port");
let name: Option<String> = xconfig::get_string("Server.Name");

// 反序列化到结构体
#[derive(serde::Deserialize)]
struct MyAppConfig {
    api_key: String,
    timeout: String,
}
let cfg: MyAppConfig = xconfig::parse_config("MyApp").unwrap();
```

### 链路追踪

```rust
use x_one::xtrace;

// xhttp、xorm、xlog 会自动关联 TraceID，一般无需手动操作
// 如需手动创建 Span：
let tracer = xtrace::tracer();
```

配置示例：

```yaml
XTrace:
  Enable: true
  Console: false   # 仅调试时开启控制台输出
```

## 生命周期 Hook

```rust
use x_one::xhook::HookOptions;

// 服务启动前执行（在所有模块初始化之后）
x_one::before_start!(|| {
    println!("Custom initialization...");
    Ok(())
}, HookOptions::new().order(100));

// 服务停止前执行
x_one::before_stop!(|| {
    println!("Cleaning up resources...");
    Ok(())
}, HookOptions::new().order(50));
```

也可手动控制生命周期：

```rust
// 手动初始化所有模块
x_one::init().await?;

// 你的业务逻辑...

// 手动清理
x_one::shutdown().ok();
```

## 多环境配置

通过 Profile 加载不同环境的配置文件：

```yaml
# application.yml（公共配置）
Server:
  Name: "my-service"
  Profiles:
    Active: "dev"    # 指定环境
```

框架会按顺序加载：`application.yml` → `application-dev.yml`，后者覆盖前者同名配置。

也可通过环境变量指定：

```bash
export SERVER_PROFILES_ACTIVE=prod
```

## 环境变量

| 环境变量 | 说明 | 示例 |
|---|---|---|
| `SERVER_ENABLE_DEBUG` | 启用框架内部调试日志 | `true` |
| `SERVER_PROFILES_ACTIVE` | 指定激活的配置环境 | `dev`, `prod` |
| `SERVER_CONFIG_LOCATION` | 指定配置文件路径 | `/app/config.yml` |

配置文件支持环境变量占位符（带默认值）：

```yaml
XOrm:
  DSN: "${DB_DSN:-mysql://user:pass@localhost:3306/db}"
```

## 完整配置参考

```yaml
Server:
  Name: "my-service"          # 服务名（必填）
  Version: "v1.0.0"           # 版本号（默认 v0.0.1）
  Profiles:
    Active: "dev"              # 环境标识

XAxum:
  Host: "0.0.0.0"             # 监听地址（默认 0.0.0.0）
  Port: 8000                   # 监听端口（默认 8000）
  UseHttp2: false              # 启用 h2c（默认 false）
  EnableBanner: true           # 打印启动 Banner（默认 true）

XLog:
  Level: "info"                # 日志级别（默认 info）
  Console: true                # 控制台打印（默认 false）
  Path: "./log/"               # 日志文件夹（默认 ./log/）
  MaxAge: "7d"                 # 日志保留时长（默认 7d）
  RotateTime: "1d"             # 切割周期（默认 1d）

XTrace:
  Enable: true                 # 启用链路追踪（默认 true）
  Console: false               # 控制台打印（默认 false）

XHttp:
  Timeout: "30s"               # 请求超时（默认 30s）
  RetryCount: 3                # 重试次数（默认 0）
  MaxIdleConnsPerHost: 100     # 每 host 最大空闲连接（默认 100）

XOrm:
  Driver: "postgres"           # 驱动（默认 postgres）
  DSN: ""                      # 连接字符串（必填）
  MaxOpenConns: 100            # 最大连接数（默认 100）
  EnableLog: true              # 开启 SQL 日志（默认 true）
  SlowThreshold: "200ms"       # 慢查询阈值（默认 200ms）

XCache:
  MaxCapacity: 100000          # 最大条目数（默认 100000）
  DefaultTTL: "5m"             # 默认 TTL（默认 5m）
```

## 更新日志

- **v0.2.11** (2026-02-11) - 改善 hook debug 日志（显示函数名代替文件路径）
- **v0.2.10** (2026-02-11) - 重命名 debug 环境变量（SERVER_ENABLE_DEBUG → XONE_ENABLE_DEBUG）
- **v0.2.9** (2026-02-11) - 优化 xhttp/xaxum 热路径性能（减少每请求堆分配、零分配比较、复用 Builder）
- **v0.2.8** (2026-02-11) - 优化 .claude 配置（新增 agents 知识库、对齐 skills、完善 README 和 Schema）
- **v0.2.7** (2026-02-10) - 发布到 crates.io
- **v0.2.6** (2026-02-10) - 美化启动 Banner（雾蓝→淡紫渐变色）+ 清理未使用导入
- **v0.2.5** (2026-02-10) - 回退日志中间件异步 spawn（高 QPS 下调度开销大于格式化开销），保留同步优化
- **v0.2.4** (2026-02-10) - 日志中间件性能优化（手动 JSON 拼接、不缓冲响应 body）+ 全局 pub 可见性治理
- **v0.2.2** (2026-02-10) - 完善 /publish 发布流程（版本 tag 检测）
- **v0.2.1** (2026-02-10) - 修复 graceful shutdown 被绕过、profile 合并丢失 Server 配置、空字符串 Duration 解析绕过默认值
- **v0.2.0** (2026-02-10) - xaxum 支持 h2c (HTTP/2 cleartext)、日志增加 caller/threadName 字段、HTTP 请求耗时人性化显示、幂等 Hook 注册
- **v0.1.0** (2026-02-10) - 初始版本，11 个模块（xconfig, xlog, xtrace, xhttp, xorm, xcache, xaxum, xhook, xserver, xutil, error）
