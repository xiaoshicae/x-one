//! 配置结构体定义

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

    /// HTTP 服务相关配置
    #[serde(rename = "Auxm")]
    pub auxm: Option<AuxmConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "v0.0.1".to_string(),
            profiles: None,
            auxm: None,
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

/// Auxm HTTP 服务配置（基于 Axum）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuxmConfig {
    /// 服务监听的 host（默认 "0.0.0.0"）
    #[serde(rename = "Host")]
    pub host: String,

    /// 服务端口号（默认 8000）
    #[serde(rename = "Port")]
    pub port: u16,

    /// 是否使用 HTTP/2 协议
    #[serde(rename = "UseHttp2")]
    pub use_http2: bool,

    /// Swagger 相关配置
    #[serde(rename = "Swagger")]
    pub swagger: Option<AuxmSwaggerConfig>,
}

impl Default for AuxmConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8000,
            use_http2: false,
            swagger: None,
        }
    }
}

/// Auxm Swagger 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuxmSwaggerConfig {
    /// 提供 API 服务的 host
    #[serde(rename = "Host")]
    pub host: String,

    /// API 公共前缀
    #[serde(rename = "BasePath")]
    pub base_path: String,

    /// API 管理后台标题
    #[serde(rename = "Title")]
    pub title: String,

    /// API 管理后台描述
    #[serde(rename = "Description")]
    pub description: String,

    /// API 支持的协议（默认 ["https", "http"]）
    #[serde(rename = "Schemes")]
    pub schemes: Vec<String>,
}

impl Default for AuxmSwaggerConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            base_path: String::new(),
            title: String::new(),
            description: String::new(),
            schemes: vec!["https".to_string(), "http".to_string()],
        }
    }
}
