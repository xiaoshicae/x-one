//! xconfig - 配置管理模块
//!
//! 提供 YAML 配置文件加载、多环境 profile 支持、
//! 环境变量占位符展开等功能。

pub mod config;
pub mod env_expand;
pub mod init;
pub mod location;
pub mod profiles;

pub use config::{AuxmConfig, AuxmSwaggerConfig, ProfilesConfig, SERVER_CONFIG_KEY, ServerConfig};

use parking_lot::RwLock;
use std::sync::OnceLock;

/// 全局配置存储
static CONFIG: OnceLock<RwLock<Option<serde_yaml::Value>>> = OnceLock::new();

fn config_store() -> &'static RwLock<Option<serde_yaml::Value>> {
    CONFIG.get_or_init(|| RwLock::new(None))
}

/// 初始化配置系统，注册到 xhook
pub fn register_hook() {
    crate::before_start!(|| init_store(), crate::xhook::HookOptions::with_order(1));
}

/// 初始化配置存储（供框架内部自动初始化使用）
pub fn init_store() -> Result<(), crate::error::XOneError> {
    let config = init::init_xconfig().map_err(crate::error::XOneError::Config)?;
    let mut store = config_store().write();
    *store = config;
    Ok(())
}

/// 获取配置值（点分 key 路径访问）
pub fn get_value(key: &str) -> Option<serde_yaml::Value> {
    let store = config_store().read();
    let config = store.as_ref()?;

    let keys: Vec<&str> = key.split('.').collect();
    let mut current = config;
    for k in keys {
        match current.get(k) {
            Some(v) => current = v,
            None => return None,
        }
    }
    Some(current.clone())
}

/// 获取字符串配置值
pub fn get_string(key: &str) -> String {
    get_value(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// 获取布尔配置值
pub fn get_bool(key: &str) -> bool {
    get_value(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// 获取整数配置值
pub fn get_int(key: &str) -> i64 {
    get_value(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

/// 获取浮点数配置值
pub fn get_float64(key: &str) -> f64 {
    get_value(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// 获取字符串切片配置值
pub fn get_string_slice(key: &str) -> Vec<String> {
    get_value(key)
        .and_then(|v| {
            v.as_sequence().map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// 检查配置中是否包含指定 key
pub fn contain_key(key: &str) -> bool {
    get_value(key).is_some()
}

/// 解析配置值到指定类型
pub fn parse_config<T: serde::de::DeserializeOwned>(
    key: &str,
) -> Result<T, crate::error::XOneError> {
    let value = get_value(key)
        .ok_or_else(|| crate::error::XOneError::Config(format!("config key [{key}] not found")))?;
    serde_yaml::from_value(value).map_err(|e| {
        crate::error::XOneError::Config(format!("parse config [{key}] failed, err=[{e}]"))
    })
}

pub(crate) fn parse_config_list<T: serde::de::DeserializeOwned>(key: &str) -> Vec<T> {
    if let Ok(config) = parse_config::<T>(key) {
        return vec![config];
    }
    if let Ok(configs) = parse_config::<Vec<T>>(key) {
        return configs;
    }
    Vec::new()
}

// ============ Server 相关配置获取 ============

/// 默认服务名
pub const DEFAULT_SERVER_NAME: &str = "unknown.unknown.unknown";
/// 默认服务版本
pub const DEFAULT_SERVER_VERSION: &str = "v0.0.1";

/// 获取服务名（未配置时返回默认值）
pub fn get_server_name() -> String {
    crate::xutil::take_or_default(
        get_string(&format!("{SERVER_CONFIG_KEY}.Name")),
        DEFAULT_SERVER_NAME,
    )
}

/// 获取原始服务名（未配置时返回空字符串）
pub fn get_raw_server_name() -> String {
    get_string(&format!("{SERVER_CONFIG_KEY}.Name"))
}

/// 获取服务版本（未配置时返回默认值）
pub fn get_server_version() -> String {
    crate::xutil::take_or_default(
        get_string(&format!("{SERVER_CONFIG_KEY}.Version")),
        DEFAULT_SERVER_VERSION,
    )
}

/// 获取 Auxm 配置
pub fn get_auxm_config() -> AuxmConfig {
    let auxm_key = format!("{SERVER_CONFIG_KEY}.Auxm");
    parse_config::<AuxmConfig>(&auxm_key).unwrap_or_default()
}

/// 获取 Auxm Swagger 配置
pub fn get_auxm_swagger_config() -> AuxmSwaggerConfig {
    let key = format!("{SERVER_CONFIG_KEY}.Auxm.Swagger");
    parse_config::<AuxmSwaggerConfig>(&key).unwrap_or_default()
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
