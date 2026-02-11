# x-one 项目规范

## 项目概况
Go 三方库集成框架 xone 的 Rust 移植版。提供配置管理、日志、链路追踪、HTTP 客户端、
数据库连接管理、本地缓存等开箱即用的能力，通过 Hook 生命周期统一管理初始化和清理。

- Rust 2024 edition，lib crate
- 入口：`src/lib.rs`
- 示例：`examples/basic.rs`

## 语言规定
- 对话、代码注释、文档注释：简体中文
- 错误消息（`XOneError`、`tracing` 日志）：英文
- commit message：英文

## 模块地图

```
lib.rs
├── error        # 统一错误类型 XOneError + Result<T>
├── xutil        # 工具集（Duration 转换、重试、环境变量、文件等）
├── xhook        # Hook 生命周期管理（before_start!/before_stop! 宏）
├── xconfig      # YAML 配置管理（Profile 合并 + 环境变量展开）
├── xlog         # 结构化日志（tracing + JSON 文件 + 控制台）
├── xserver      # 服务器抽象（Server trait + init/shutdown 流程）
├── xtrace       # 链路追踪（OpenTelemetry）
├── xhttp        # HTTP 客户端（reqwest 封装）
├── xorm         # 数据库连接池（sqlx，Postgres/MySQL）
├── xcache       # 本地缓存（moka，类型擦除泛型缓存）
└── xaxum        # Axum HTTP 服务器（构建器 + 中间件）
```

## Hook 生命周期顺序

### BeforeStart（初始化，order 小→大）
```
xconfig(10) → xtrace(20) → xlog(30) → xhttp(40) → xorm(50) → xcache(60)
```
用户默认 order=100，可用 1-9（系统前）、15/25...（系统间）、100+（系统后）

### BeforeStop（清理，order 小→大，系统用 `i32::MAX - init_order` 保证倒序）
```
用户(100) → xcache(MAX-60) → xorm(MAX-50) → xlog(MAX-30) → xtrace(MAX-20)
```
用户 hook 先执行，系统按初始化倒序关闭，order 值极大不会与用户冲突

> xhttp 无需 before_stop hook（reqwest Client 可直接 drop，无资源需显式清理）

## 核心设计模式

### 全局状态
`OnceLock<RwLock<T>>` 或 `OnceLock<T>`，通过 `pub(crate) fn xxx_store()` 内部访问。

### 幂等 Hook 注册
每个模块 `mod.rs` 的 `register_hook()` 使用 `AtomicBool + compare_exchange` 保证只注册一次。

### 标准子模块职责（推荐基线，按需增减）
| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块声明 + pub use + register_hook() |
| `config.rs` | 配置结构体（Deserialize） |
| `client.rs` | 全局状态 + 对外查询 API（如 xconfig 用 `accessor.rs`） |
| `init.rs` | 初始化/关闭逻辑 |

参考标杆：`xorm/mod.rs`。xaxum 等模块可按需使用 `builder.rs`、`middleware/` 等替代结构。

## 开发方法
- 严格遵循 TDD 红-绿-重构循环（详见 agents/tdd-guide.md）
- 编码规范详见 rules/rust-coding.md
- 测试规范详见 rules/testing.md
- 架构规范详见 rules/architecture.md
- 版本号管理详见 rules/versioning.md

## Agent 知识库

`agents/` 目录包含专家级领域知识，供 skills 引用：

| Agent | 用途 |
|-------|------|
| `build-error-resolver` | Rust 编译/clippy 错误模式与修复策略 |
| `code-reviewer` | 增量代码审查清单（P0/P1/P2 优先级） |
| `security-reviewer` | 安全审查（unsafe/依赖/密钥/并发） |
| `tdd-guide` | TDD 完整指南（详细示例与边界测试） |

## 常用命令
```bash
cargo test                          # 运行所有测试
cargo clippy -- -D warnings         # 静态分析（零警告）
cargo fmt                           # 格式化
cargo test -- --test-threads=1      # 串行运行（调试全局状态问题）
```

## 自定义 Skills

- `/commit` - 规范提交（测试 + 覆盖率 + 版本号 + 提交）
- `/test [模块]` - 运行测试
- `/new-module <模块名> <三方库>` - 创建新模块
- `/review <模块名>` - 模块代码审查
- `/build-fix` - 快速修复编译错误
- `/publish` - 发布到 crates.io

## 关键依赖
| 用途 | crate |
|------|-------|
| 异步运行时 | tokio |
| HTTP 框架 | axum |
| 序列化 | serde + serde_yaml + serde_json |
| 日志 | tracing + tracing-subscriber + tracing-appender |
| 链路追踪 | opentelemetry 0.28 + opentelemetry_sdk |
| HTTP 客户端 | reqwest |
| 数据库 | sqlx（postgres, mysql） |
| 缓存 | moka |
| 错误处理 | thiserror |
| 并发锁 | parking_lot |
| 重试 | backon |
| 测试辅助 | serial_test, tempfile, tokio-test |