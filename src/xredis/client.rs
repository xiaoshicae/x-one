//! xredis 对外 API
//!
//! 提供 Redis 连接管理，支持单/多实例模式。
//!
//! ```ignore
//! // 获取默认 Redis 连接
//! let client = x_one::xredis::redis_client().unwrap();
//!
//! // 多实例：获取命名连接
//! let cache = x_one::xredis::redis_client_with_name("cache").unwrap();
//! ```

use parking_lot::RwLock;
use redis::aio::ConnectionManager;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 默认实例名
pub const DEFAULT_CLIENT_NAME: &str = "_default_";

/// 全局连接管理器存储
static CLIENT_STORE: OnceLock<RwLock<HashMap<String, ConnectionManager>>> = OnceLock::new();

pub(crate) fn client_store() -> &'static RwLock<HashMap<String, ConnectionManager>> {
    CLIENT_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 获取默认 Redis 连接管理器
///
/// 返回默认名称（`_default_`）对应的连接管理器。
/// `ConnectionManager` 内部基于 Arc，Clone 开销极小。
pub fn redis_client() -> Option<ConnectionManager> {
    redis_client_with_name(DEFAULT_CLIENT_NAME)
}

/// 获取命名 Redis 连接管理器
///
/// 根据名称查找对应连接管理器。
pub fn redis_client_with_name(name: &str) -> Option<ConnectionManager> {
    let store = client_store().read();
    store.get(name).cloned()
}

/// 获取所有 Redis 实例名称
pub fn get_client_names() -> Vec<String> {
    let store = client_store().read();
    store.keys().cloned().collect()
}

/// 重置连接存储（仅测试用）
#[doc(hidden)]
pub fn reset_clients() {
    client_store().write().clear();
}

/// 设置连接管理器（仅测试用）
#[doc(hidden)]
pub fn set_client(name: &str, client: ConnectionManager) {
    client_store().write().insert(name.to_string(), client);
}
