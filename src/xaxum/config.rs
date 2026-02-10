//! xaxum 配置结构体

use serde::{Deserialize, Serialize};

/// XAxum 配置 key
pub const XAXUM_CONFIG_KEY: &str = "XAxum";

/// Axum HTTP 服务配置
///
/// # 配置示例
/// ```yaml
/// XAxum:
///   Host: "0.0.0.0"
///   Port: 8000
///   UseHttp2: false
///   EnableBanner: true
///   Swagger:
///     Schemes: ["https", "http"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AxumConfig {
    /// 服务监听的 host（默认 "0.0.0.0"）
    #[serde(rename = "Host")]
    pub host: String,

    /// 服务端口号（默认 8000）
    #[serde(rename = "Port")]
    pub port: u16,

    /// 是否使用 HTTP/2 协议
    #[serde(rename = "UseHttp2")]
    pub use_http2: bool,

    /// 是否启用启动 Banner（默认 true）
    #[serde(rename = "EnableBanner")]
    pub enable_banner: bool,

    /// Swagger 相关配置
    #[serde(rename = "Swagger")]
    pub swagger: Option<AxumSwaggerConfig>,
}

impl Default for AxumConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8000,
            use_http2: false,
            enable_banner: true,
            swagger: None,
        }
    }
}

/// Axum Swagger 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AxumSwaggerConfig {
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

impl Default for AxumSwaggerConfig {
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

/// 加载 Axum 配置
pub(crate) fn load_config() -> AxumConfig {
    crate::xconfig::parse_config::<AxumConfig>(XAXUM_CONFIG_KEY).unwrap_or_default()
}
