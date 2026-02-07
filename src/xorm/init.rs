//! xorm 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::config::{Driver, LEGACY_XGORM_CONFIG_KEY, XORM_CONFIG_KEY, XOrmConfig};

/// 默认实例名
pub const DEFAULT_POOL_NAME: &str = "_default_";

/// 存储所有数据库连接池配置信息（不实际连接，仅保存配置）
/// 实际连接需要异步环境，在 before_start hook（同步）中仅做配置验证和保存
static POOL_CONFIGS: OnceLock<RwLock<HashMap<String, PoolEntry>>> = OnceLock::new();

pub fn pool_configs_store() -> &'static RwLock<HashMap<String, PoolEntry>> {
    POOL_CONFIGS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 连接池条目
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// 数据库配置
    pub config: XOrmConfig,
    /// 是否已连接（异步初始化后设为 true）
    pub connected: bool,
}

/// 初始化 XOrm（验证配置并保存）
pub fn init_xorm() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XORM_CONFIG_KEY) && !xconfig::contain_key(LEGACY_XGORM_CONFIG_KEY) {
        xutil::info_if_enable_debug("XOrm config not found, skip init");
        return Ok(());
    }

    let configs = load_configs();

    if configs.is_empty() {
        xutil::info_if_enable_debug("XOrm config empty, skip init");
        return Ok(());
    }

    let mut store = pool_configs_store().write();

    for config in configs {
        if config.dsn.is_empty() {
            xutil::warn_if_enable_debug(&format!(
                "XOrm config name=[{}] has empty DSN, skip",
                config.name
            ));
            continue;
        }

        let name = xutil::default_if_empty(&config.name, DEFAULT_POOL_NAME).to_string();

        xutil::info_if_enable_debug(&format!(
            "XOrm register pool name=[{}], driver=[{}], max_open=[{}], max_idle=[{}]",
            name, config.driver, config.max_open_conns, config.max_idle_conns
        ));

        store.insert(
            name,
            PoolEntry {
                config,
                connected: false,
            },
        );
    }

    xutil::info_if_enable_debug(&format!("XOrm init success, pool_count=[{}]", store.len()));
    Ok(())
}

/// 关闭所有连接池
pub fn shutdown_xorm() -> Result<(), crate::error::XOneError> {
    let mut store = pool_configs_store().write();
    let count = store.len();
    store.clear();
    xutil::info_if_enable_debug(&format!("XOrm shutdown, cleared {count} pool configs"));
    Ok(())
}

/// 获取连接池配置
pub fn get_pool_config(name: Option<&str>) -> Option<XOrmConfig> {
    let store = pool_configs_store().read();
    let key = name.unwrap_or(DEFAULT_POOL_NAME);
    store.get(key).map(|entry| entry.config.clone())
}

/// 获取所有连接池名称
pub fn get_pool_names() -> Vec<String> {
    let store = pool_configs_store().read();
    store.keys().cloned().collect()
}

/// 获取驱动类型
pub fn get_driver(name: Option<&str>) -> Option<Driver> {
    get_pool_config(name).map(|c| c.driver)
}

/// 获取 DSN
pub fn get_dsn(name: Option<&str>) -> Option<String> {
    get_pool_config(name).map(|c| c.dsn)
}

/// 加载 XOrm 配置（支持单实例和多实例模式）
pub fn load_configs() -> Vec<XOrmConfig> {
    let configs = xconfig::parse_config_list::<XOrmConfig>(XORM_CONFIG_KEY);
    if !configs.is_empty() {
        return configs;
    }

    xconfig::parse_config_list::<XOrmConfig>(LEGACY_XGORM_CONFIG_KEY)
}
