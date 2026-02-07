//! xcache 配置结构体

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

impl Default for XCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: default_max_capacity(),
            default_ttl: default_ttl(),
            name: String::new(),
        }
    }
}