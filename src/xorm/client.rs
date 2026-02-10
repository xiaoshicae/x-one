//! xorm 对外 API
//!
//! 提供数据库连接池管理，支持 Postgres / MySQL 类型化连接池。
//!
//! ```ignore
//! // 获取默认 Postgres 连接池
//! let pool = x_one::xorm::db().unwrap();
//! let pg = pool.as_postgres().unwrap();
//! let row = sqlx::query("SELECT 1").fetch_one(pg).await?;
//!
//! // 多数据源：获取命名连接池
//! let analytics = x_one::xorm::db_with_name("analytics").unwrap();
//! ```

use super::config::Driver;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 默认实例名
pub const DEFAULT_POOL_NAME: &str = "_default_";

/// 数据库连接池（类型化）
///
/// 通过 `as_postgres()` / `as_mysql()` 获取对应类型的连接池引用。
/// Pool 内部基于 Arc，Clone 开销极小。
#[derive(Clone, Debug)]
pub enum DbPool {
    /// PostgreSQL 连接池
    Postgres(sqlx::Pool<sqlx::Postgres>),
    /// MySQL 连接池
    MySql(sqlx::Pool<sqlx::MySql>),
}

impl DbPool {
    /// 获取 PostgreSQL 连接池引用
    ///
    /// 如果是 Postgres 变体返回 `Some`，否则 `None`。
    pub fn as_postgres(&self) -> Option<&sqlx::Pool<sqlx::Postgres>> {
        match self {
            DbPool::Postgres(pool) => Some(pool),
            _ => None,
        }
    }

    /// 获取 MySQL 连接池引用
    ///
    /// 如果是 MySql 变体返回 `Some`，否则 `None`。
    pub fn as_mysql(&self) -> Option<&sqlx::Pool<sqlx::MySql>> {
        match self {
            DbPool::MySql(pool) => Some(pool),
            _ => None,
        }
    }

    /// 获取驱动类型
    pub fn driver(&self) -> Driver {
        match self {
            DbPool::Postgres(_) => Driver::Postgres,
            DbPool::MySql(_) => Driver::Mysql,
        }
    }
}

/// 全局连接池存储
static POOL_STORE: OnceLock<RwLock<HashMap<String, DbPool>>> = OnceLock::new();

pub(crate) fn pool_store() -> &'static RwLock<HashMap<String, DbPool>> {
    POOL_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 获取默认连接池
///
/// 返回默认名称（`_default_`）对应的连接池。
/// Pool 内部基于 Arc，Clone 开销极小。
pub fn db() -> Option<DbPool> {
    db_with_name(DEFAULT_POOL_NAME)
}

/// 获取命名连接池
///
/// 根据名称查找对应连接池。
/// Pool 内部基于 Arc，Clone 开销极小。
pub fn db_with_name(name: &str) -> Option<DbPool> {
    let store = pool_store().read();
    store.get(name).cloned()
}

/// 获取所有连接池名称
pub fn get_pool_names() -> Vec<String> {
    let store = pool_store().read();
    store.keys().cloned().collect()
}

/// 重置连接池存储（仅测试用）
#[doc(hidden)]
pub fn reset_pools() {
    pool_store().write().clear();
}

/// 设置连接池（仅测试用）
#[doc(hidden)]
pub fn set_pool(name: &str, pool: DbPool) {
    pool_store().write().insert(name.to_string(), pool);
}
