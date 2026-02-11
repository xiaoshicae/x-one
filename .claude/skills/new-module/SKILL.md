# 新增模块

快速接入新的三方库到 x-one 框架。

使用方法: /new-module <模块名> <三方库名>

示例:
- /new-module xredis redis-rs
- /new-module xmongo mongodb

$ARGUMENTS

---

请按照 x-one 框架规范创建新模块，包含以下文件：

## 1. 目录结构

```
src/x{模块名}/
├── mod.rs          # 模块声明 + pub use + register_hook()
├── config.rs       # 配置结构体（Deserialize）
├── client.rs       # 全局状态 + 对外查询 API
└── init.rs         # 初始化/关闭逻辑

tests/x{模块名}/
├── mod.rs          # 子模块声明
├── config.rs       # 配置结构体测试
├── client.rs       # API 测试
└── init.rs         # 初始化集成测试

tests/x{模块名}.rs  # 顶层入口，用 #[path] 导入子模块
```

## 2. 编码规范

### mod.rs
```rust
//! x{模块名} 模块 - {简短描述}

use std::sync::atomic::{AtomicBool, Ordering};

mod config;
mod client;
mod init;

pub use config::*;
pub use client::*;

static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册 x{模块名} 模块的 Hook
pub fn register_hook() {
    if REGISTERED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    crate::before_start!(init::init_x{模块名}, crate::xhook::HookOptions::new().order(N));
    crate::before_stop!(init::shutdown_x{模块名}, crate::xhook::HookOptions::new().order(i32::MAX - N));
}
```

### config.rs
```rust
/// x{模块名} 配置
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct X{模块名}Config {
    // 配置字段...
}

/// 配置键名
pub const X{模块名}_CONFIG_KEY: &str = "x-{模块名}";
```

### client.rs
```rust
use std::sync::OnceLock;
use parking_lot::RwLock;

static STORE: OnceLock<RwLock<Option<Client>>> = OnceLock::new();

pub(crate) fn store() -> &'static RwLock<Option<Client>> {
    STORE.get_or_init(|| RwLock::new(None))
}

/// 获取客户端
pub fn client() -> Option<Client> {
    store().read().clone()
}

/// 重置（仅测试用）
#[doc(hidden)]
pub fn reset() {
    *store().write() = None;
}
```

### init.rs
```rust
use crate::error::XOneError;

pub(crate) fn init_x{模块名}() -> Result<(), XOneError> {
    if !crate::xconfig::contains_key(super::config::X{模块名}_CONFIG_KEY) {
        return Ok(());
    }
    // 初始化逻辑...
    Ok(())
}

pub(crate) fn shutdown_x{模块名}() -> Result<(), XOneError> {
    // 清理逻辑...
    Ok(())
}
```

## 3. 注意事项

- 时间配置使用 `xutil::to_duration()`
- 全局状态使用 `OnceLock<RwLock<T>>` + `parking_lot`
- 错误使用 `XOneError` 变体，禁止直接 `panic!`
- 代码注释使用中文，错误消息使用英文
- 测试全局状态用 `#[serial]` + `reset()`
- 异步测试用 `#[tokio::test]`

## 4. 在 lib.rs 中注册

在 `src/lib.rs` 添加：
```rust
pub mod x{模块名};
```

请根据以上模板创建模块代码，并以 TDD 方式逐步实现。