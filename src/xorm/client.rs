//! xorm 对外 API
//!
//! 提供连接池配置查询、驱动类型获取、DSN 获取等功能。

use super::config::{Driver, XOrmConfig};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 默认实例名
pub const DEFAULT_POOL_NAME: &str = "_default_";

/// 连接池条目
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// 数据库配置
    pub config: XOrmConfig,
}

/// 存储所有数据库连接池配置信息（不实际连接，仅保存配置）
static POOL_CONFIGS: OnceLock<RwLock<HashMap<String, PoolEntry>>> = OnceLock::new();

pub fn pool_configs_store() -> &'static RwLock<HashMap<String, PoolEntry>> {
    POOL_CONFIGS.get_or_init(|| RwLock::new(HashMap::new()))
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
