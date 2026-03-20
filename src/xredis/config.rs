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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_load_configs_no_config_returns_empty() {
        crate::xconfig::reset_config();
        let result = load_configs();
        assert!(result.is_empty(), "无配置时应返回空列表");
        crate::xconfig::reset_config();
    }

    #[test]
    #[serial]
    fn test_load_configs_single_instance() {
        crate::xconfig::reset_config();
        let yaml = r#"XRedis:
  Addr: "redis://myhost:6380"
  Name: "cache"
"#;
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        crate::xconfig::set_config(config);
        let result = load_configs();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, "redis://myhost:6380");
        assert_eq!(result[0].name, "cache");
        crate::xconfig::reset_config();
    }

    #[test]
    #[serial]
    fn test_load_configs_multiple_instances() {
        crate::xconfig::reset_config();
        let yaml = r#"XRedis:
  - Addr: "redis://host1:6379"
    Name: "first"
  - Addr: "redis://host2:6379"
    Name: "second"
"#;
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        crate::xconfig::set_config(config);
        let result = load_configs();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "first");
        assert_eq!(result[1].name, "second");
        crate::xconfig::reset_config();
    }

    #[test]
    fn test_sanitize_for_log_with_password() {
        let config = XRedisConfig {
            password: "secret123".to_string(),
            ..Default::default()
        };
        let sanitized = sanitize_for_log(&config);
        assert_eq!(sanitized.password, "***", "密码应被脱敏");
        // 其他字段保持不变
        assert_eq!(sanitized.addr, config.addr);
        assert_eq!(sanitized.name, config.name);
    }

    #[test]
    fn test_sanitize_for_log_without_password() {
        let config = XRedisConfig::default();
        let sanitized = sanitize_for_log(&config);
        assert_eq!(sanitized.password, "", "空密码不应被替换");
    }
}
