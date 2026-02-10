//! 配置结构体定义
//!
//! 包含 `ServerConfig` 和 `ProfilesConfig`，对应 `application.yml` 中的 `Server` 节点。
//!
//! ```yaml
//! Server:
//!   Name: "my-app"
//!   Version: "v1.0.0"
//!   Profiles:
//!     Active: "dev"
//! ```

use serde::{Deserialize, Serialize};

/// Server 配置 key
pub const SERVER_CONFIG_KEY: &str = "Server";

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// 服务名（必填）
    #[serde(rename = "Name")]
    pub name: String,

    /// 服务版本号（默认 "v0.0.1"）
    #[serde(rename = "Version")]
    pub version: String,

    /// 环境相关配置
    #[serde(rename = "Profiles")]
    pub profiles: Option<ProfilesConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "v0.0.1".to_string(),
            profiles: None,
        }
    }
}

/// 多环境配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfilesConfig {
    /// 指定启用的环境
    #[serde(rename = "Active")]
    pub active: String,
}
