//! xorm 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;

use super::client::{DEFAULT_POOL_NAME, DbPool, pool_store};
use super::config::{Driver, XORM_CONFIG_KEY, XOrmConfig};

/// 初始化 XOrm（根据配置创建连接池）
///
/// 读取 YAML 配置，为每个有效配置创建 lazy 连接池并存入全局 store。
/// `connect_lazy` 只解析 URL 不建立连接，兼容同步 Hook 系统。
pub fn init_xorm() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XORM_CONFIG_KEY) {
        xutil::info_if_enable_debug("XOrm config not found, skip init");
        return Ok(());
    }

    let configs = super::config::load_configs();

    if configs.is_empty() {
        xutil::info_if_enable_debug("XOrm config empty, skip init");
        return Ok(());
    }

    let mut store = pool_store().write();

    for config in configs {
        if config.dsn.is_empty() {
            xutil::warn_if_enable_debug(&format!(
                "XOrm config name=[{}] has empty DSN, skip",
                config.name
            ));
            continue;
        }

        let name = xutil::default_if_empty(config.name.as_str(), DEFAULT_POOL_NAME).to_string();

        let pool = build_pool(&config)?;

        xutil::info_if_enable_debug(&format!(
            "XOrm register pool name=[{}], driver=[{}], max_open=[{}], max_idle=[{}]",
            name, config.driver, config.max_open_conns, config.max_idle_conns
        ));

        store.insert(name, pool);
    }

    xutil::info_if_enable_debug(&format!("XOrm init success, pool_count=[{}]", store.len()));
    Ok(())
}

/// 根据配置构建 lazy 连接池
///
/// 使用 `connect_lazy` 创建连接池，只解析 URL 不建立真实连接。
#[doc(hidden)]
pub fn build_pool(config: &XOrmConfig) -> Result<DbPool, crate::error::XOneError> {
    let max_lifetime = xutil::to_duration(&config.max_lifetime);
    let idle_timeout = xutil::to_duration(&config.max_idle_time);

    match config.driver {
        Driver::Postgres => {
            let pool = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
                .max_connections(config.max_open_conns)
                .min_connections(config.max_idle_conns)
                .max_lifetime(max_lifetime)
                .idle_timeout(idle_timeout)
                .connect_lazy(&config.dsn)
                .map_err(|e| {
                    crate::error::XOneError::Other(format!(
                        "XOrm build postgres pool failed: {}",
                        e
                    ))
                })?;
            Ok(DbPool::Postgres(pool))
        }
        Driver::Mysql => {
            let pool = sqlx::pool::PoolOptions::<sqlx::MySql>::new()
                .max_connections(config.max_open_conns)
                .min_connections(config.max_idle_conns)
                .max_lifetime(max_lifetime)
                .idle_timeout(idle_timeout)
                .connect_lazy(&config.dsn)
                .map_err(|e| {
                    crate::error::XOneError::Other(format!("XOrm build mysql pool failed: {}", e))
                })?;
            Ok(DbPool::MySql(pool))
        }
    }
}

/// 关闭所有连接池
///
/// 清空全局 store，Pool drop 时自动清理连接。
pub fn shutdown_xorm() -> Result<(), crate::error::XOneError> {
    let mut store = pool_store().write();
    let count = store.len();
    store.clear();
    xutil::info_if_enable_debug(&format!("XOrm shutdown, cleared {count} pools"));
    Ok(())
}
