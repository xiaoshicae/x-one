//! xcache 配置结构体
//!
//! 对应 `application.yml` 中的 `XCache` 节点，支持单实例和多实例配置。
//!
//! ```yaml
//! # 单实例（使用默认名称）
//! XCache:
//!   MaxCapacity: 100000
//!   DefaultTTL: "5m"
//!
//! # 多实例
//! XCache:
//!   - MaxCapacity: 50000
//!     DefaultTTL: "5m"
//!     Name: "session"
//!   - MaxCapacity: 10000
//!     DefaultTTL: "1h"
//!     Name: "user"
//! ```

use serde::Deserialize;

/// XCache 配置 key
pub const XCACHE_CONFIG_KEY: &str = "XCache";

/// XCache 配置
///
/// # 配置示例
/// ```yaml
/// XCache:
///   MaxCapacity: 100000
///   DefaultTTL: "5m"
///   Name: ""
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct XCacheConfig {
    /// 最大缓存容量（对应 moka 的 max_capacity，默认 100_000）
    #[serde(rename = "MaxCapacity", default = "default_max_capacity")]
    pub max_capacity: u64,

    /// 默认 TTL（duration 字符串，默认 "5m"）
    #[serde(rename = "DefaultTTL", default = "default_ttl")]
    pub default_ttl: String,

    /// 实例名称（多实例模式标识，默认空）
    #[serde(rename = "Name", default)]
    pub name: String,
}

fn default_max_capacity() -> u64 {
    100_000
}
fn default_ttl() -> String {
    "5m".to_string()
}

/// 加载 XCache 配置（支持单实例和多实例模式）
pub(crate) fn load_configs() -> Vec<XCacheConfig> {
    crate::xconfig::parse_config_list::<XCacheConfig>(XCACHE_CONFIG_KEY)
}

impl Default for XCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: default_max_capacity(),
            default_ttl: default_ttl(),
            name: String::new(),
        }
    }
}
