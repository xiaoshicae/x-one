//! xorm 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;

use super::client::{DEFAULT_POOL_NAME, PoolEntry, pool_configs_store};
use super::config::{XORM_CONFIG_KEY, XOrmConfig};

/// 初始化 XOrm（验证配置并保存）
pub fn init_xorm() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XORM_CONFIG_KEY) {
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

        let name = xutil::default_if_empty(config.name.as_str(), DEFAULT_POOL_NAME).to_string();

        xutil::info_if_enable_debug(&format!(
            "XOrm register pool name=[{}], driver=[{}], max_open=[{}], max_idle=[{}]",
            name, config.driver, config.max_open_conns, config.max_idle_conns
        ));

        store.insert(name, PoolEntry { config });
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

/// 加载 XOrm 配置（支持单实例和多实例模式）
pub fn load_configs() -> Vec<XOrmConfig> {
    xconfig::parse_config_list::<XOrmConfig>(XORM_CONFIG_KEY)
}
