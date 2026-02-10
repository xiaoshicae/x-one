# 架构规范

## 模块分层

### 基础层（无外部依赖）
- **error** - 统一错误类型 `XOneError`，所有模块通过 `Result<T, XOneError>` 传播错误
- **xutil** - 纯工具函数，无全局状态，无 Hook
- **xhook** - Hook 生命周期引擎，`before_start!` / `before_stop!` 宏

### 核心层（依赖基础层）
- **xconfig** - 配置管理，所有其他模块的配置来源
- **xlog** - 日志系统，依赖 xconfig 读取配置
- **xserver** - 服务器抽象，协调 init → run → shutdown 流程

### 集成层（依赖核心层）
- **xtrace** - 链路追踪，依赖 xconfig
- **xhttp** - HTTP 客户端，依赖 xconfig
- **xorm** - 数据库连接池，依赖 xconfig
- **xcache** - 本地缓存，依赖 xconfig
- **xaxum** - Axum HTTP 服务器，依赖 xconfig + xlog + xtrace

## 全局状态管理

### 存储模式
```rust
// 多实例存储（xorm、xcache）
static STORE: OnceLock<RwLock<HashMap<String, T>>> = OnceLock::new();

// 单实例存储（xhttp）
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

// 可选值存储（xconfig）
static CONFIG: OnceLock<RwLock<Option<serde_yaml::Value>>> = OnceLock::new();
```

### 访问模式
```rust
// 内部访问（pub(crate)，放在 client.rs）
pub(crate) fn store() -> &'static RwLock<HashMap<String, T>> {
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

// 外部查询 API（pub，放在 client.rs）
pub fn get(name: &str) -> Option<T> {
    store().read().get(name).cloned()
}

// 测试重置（pub + #[doc(hidden)]，放在 client.rs）
#[doc(hidden)]
pub fn reset() { store().write().clear(); }
```

## 幂等 Hook 注册模式

每个需要 Hook 的模块在 `mod.rs` 中实现：
```rust
use std::sync::atomic::{AtomicBool, Ordering};

static REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn register_hook() {
    if REGISTERED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    crate::before_start!(init::init_xxx, crate::xhook::HookOptions::new().order(N));
    crate::before_stop!(init::shutdown_xxx, crate::xhook::HookOptions::new().order(M));
}
```

## 配置结构体模式

```rust
// config.rs
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct XModuleConfig {
    pub enabled: bool,
    pub timeout: String,  // 字符串格式的 Duration，如 "30s"、"5m"
}

pub const XMODULE_CONFIG_KEY: &str = "x-module";
```

配置文件路径：`application.yml` 中的 `x-module:` 节点，
通过 `xconfig::parse_config::<XModuleConfig>(XMODULE_CONFIG_KEY)` 读取。

## 错误处理

### XOneError 变体选择
| 场景 | 变体 |
|------|------|
| Hook 执行失败 | `XOneError::Hook(msg)` |
| 配置解析/缺失 | `XOneError::Config(msg)` |
| 日志初始化失败 | `XOneError::Log(msg)` |
| 服务器运行失败 | `XOneError::Server(msg)` |
| IO 操作失败 | `XOneError::Io(err)` |
| 多个错误合并 | `XOneError::Multi(vec)` |
| 其他/初始化错误 | `XOneError::Other(msg)` |

### 错误传播
- Hook 函数签名：`fn() -> Result<(), XOneError>`
- 模块初始化：返回 `Result<(), XOneError>`，在 Hook 中调用
- 外部 API：返回 `Option<T>` 表示"可能未初始化"

## 新增模块指南

1. 创建 `src/xnew/` 目录，推荐基线文件（按需增减）：
   - `mod.rs` - 模块声明 + pub use + register_hook()（必须）
   - `config.rs` - 配置结构体（必须）
   - `client.rs` - 全局状态 + 对外 API（可用其他名称如 `accessor.rs`）
   - `init.rs` - 初始化/关闭逻辑（必须）

2. 在 `src/lib.rs` 中添加 `pub mod xnew;`

3. 分配 Hook order（BeforeStart 和 BeforeStop 各一个数字）

4. 创建 `tests/xnew/` 目录，按子模块组织测试

5. 在 `tests/xnew.rs` 中用 `#[path]` 导入子测试模块

## 测试目录约定

```
tests/
├── xmodule.rs           # 顶层入口，用 #[path] 导入子模块
└── xmodule/
    ├── mod.rs           # 子模块声明（如有共享 helper）
    ├── config.rs        # 配置结构体测试
    ├── client.rs        # API 测试
    └── init.rs          # 初始化集成测试（需要 serial_test）
```

### 全局状态测试注意
- 修改全局状态的测试必须用 `#[serial]`（来自 `serial_test` crate）
- 每个测试开始前调用 `reset_xxx()` 清理状态
- 需要 Tokio runtime 的测试用 `#[tokio::test]`

## lib.rs 公开导出

顶层 `pub use` 只导出最常用的类型，用户通过 `use x_one::xxx` 直接访问：
```rust
pub use error::{Result, XOneError};
pub use xaxum::{XAxum, XAxumServer};
pub use xserver::Server;
pub use xserver::blocking::BlockingServer;
pub use xserver::{init, run_blocking_server, run_server, shutdown};
```

其他 API 通过模块路径访问，如 `x_one::xorm::db()`、`x_one::xcache::get::<T>(key)`。