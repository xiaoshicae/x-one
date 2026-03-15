//! xredis 配置结构体
//!
//! 对应 `application.yml` 中的 `XRedis` 节点，支持单实例和多实例配置。
//!
//! ```yaml
//! # 单实例
//! XRedis:
//!   Addr: "redis://localhost:6379"
//!
//! # 多实例
//! XRedis:
//!   - Addr: "redis://localhost:6379"
//!     Name: "cache"
//!   - Addr: "redis://localhost:6380"
//!     Name: "session"
//! ```

use serde::Deserialize;

/// XRedis 配置 key
pub const XREDIS_CONFIG_KEY: &str = "XRedis";

/// XRedis 配置
#[derive(Debug, Deserialize, Clone)]
pub struct XRedisConfig {
    /// Redis 服务器地址（redis:// URL 格式）
    #[serde(rename = "Addr", default = "default_addr")]
    pub addr: String,

    /// 连接密码
    #[serde(rename = "Password", default)]
    pub password: String,

    /// 数据库编号（默认 0）
    #[serde(rename = "DB", default)]
    pub db: i64,

    /// 用户名（Redis 6.0+ ACL）
    #[serde(rename = "Username", default)]
    pub username: String,

    /// 连接超时（duration 字符串，默认 "500ms"）
    #[serde(rename = "DialTimeout", default = "default_dial_timeout")]
    pub dial_timeout: String,

    /// 读超时（duration 字符串，默认 "500ms"）
    #[serde(rename = "ReadTimeout", default = "default_read_timeout")]
    pub read_timeout: String,

    /// 写超时（duration 字符串，默认 "500ms"）
    #[serde(rename = "WriteTimeout", default = "default_write_timeout")]
    pub write_timeout: String,

    /// 最大重试次数（默认 3，设为 0 禁用重试）
    #[serde(rename = "MaxRetries", default = "default_max_retries")]
    pub max_retries: u32,

    /// 实例名称（多实例模式标识，默认空）
    #[serde(rename = "Name", default)]
    pub name: String,
}

fn default_addr() -> String {
    "redis://localhost:6379".to_string()
}
fn default_dial_timeout() -> String {
    "500ms".to_string()
}
fn default_read_timeout() -> String {
    "500ms".to_string()
}
fn default_write_timeout() -> String {
    "500ms".to_string()
}
fn default_max_retries() -> u32 {
    3
}

impl Default for XRedisConfig {
    fn default() -> Self {
        Self {
            addr: default_addr(),
            password: String::new(),
            db: 0,
            username: String::new(),
            dial_timeout: default_dial_timeout(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            max_retries: default_max_retries(),
            name: String::new(),
        }
    }
}

/// 加载 XRedis 配置（支持单实例和多实例模式）
pub(crate) fn load_configs() -> Vec<XRedisConfig> {
    crate::xconfig::parse_config_list::<XRedisConfig>(XREDIS_CONFIG_KEY)
}

/// 生成脱敏配置用于日志输出
pub(crate) fn sanitize_for_log(config: &XRedisConfig) -> XRedisConfig {
    let mut c = config.clone();
    if !c.password.is_empty() {
        c.password = "***".to_string();
    }
    c
}
