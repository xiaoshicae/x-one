//! xconfig - 配置管理模块
//!
//! 提供 YAML 配置文件加载、多环境 profile 支持、
//! 环境变量占位符展开等功能。

pub mod accessor;
pub mod config;
pub mod env_expand;
pub mod init;
pub mod location;
pub mod profiles;
pub mod server_config;

pub(crate) use accessor::parse_config_list;
pub use accessor::{
    contain_key, get_bool, get_float64, get_int, get_string, get_string_slice, get_value,
    parse_config,
};
pub use config::{ProfilesConfig, SERVER_CONFIG_KEY, ServerConfig};
pub use server_config::{
    DEFAULT_SERVER_NAME, DEFAULT_SERVER_VERSION, get_raw_server_name, get_server_name,
    get_server_version,
};

use parking_lot::RwLock;
use std::sync::OnceLock;

/// 全局配置存储
static CONFIG: OnceLock<RwLock<Option<serde_yaml::Value>>> = OnceLock::new();

pub(crate) fn config_store() -> &'static RwLock<Option<serde_yaml::Value>> {
    CONFIG.get_or_init(|| RwLock::new(None))
}

/// 初始化配置系统，注册到 xhook
pub fn register_hook() {
    crate::before_start!(init_store, crate::xhook::HookOptions::new().order(1));
}

/// 初始化配置存储（供框架内部自动初始化使用）
pub fn init_store() -> Result<(), crate::error::XOneError> {
    let config = init::init_xconfig()?;
    let mut store = config_store().write();
    *store = config;
    Ok(())
}

/// 重置配置存储（仅测试用）
pub fn reset_config() {
    let mut store = config_store().write();
    *store = None;
}

/// 设置配置（仅测试用）
pub fn set_config(config: serde_yaml::Value) {
    let mut store = config_store().write();
    *store = Some(config);
}
