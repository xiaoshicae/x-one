//! xorm 配置结构体

use serde::Deserialize;

/// XOrm 配置 key
pub const XORM_CONFIG_KEY: &str = "XOrm";

/// 数据库驱动类型
#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
pub enum Driver {
    /// PostgreSQL
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    /// MySQL
    #[serde(rename = "mysql")]
    Mysql,
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Driver::Postgres => write!(f, "postgres"),
            Driver::Mysql => write!(f, "mysql"),
        }
    }
}

/// XOrm 配置
///
/// # 配置示例
/// ```yaml
/// XOrm:
///   Driver: "postgres"
///   DSN: "postgres://user:pass@localhost:5432/db"
///   MaxOpenConns: 100
///   MaxIdleConns: 10
///   MaxLifetime: "1h"
///   MaxIdleTime: "10m"
///   SlowThreshold: "200ms"
///   EnableLog: true
///   Name: ""
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct XOrmConfig {
    /// 数据库驱动（默认 postgres）
    #[serde(rename = "Driver", default)]
    pub driver: Driver,

    /// 数据库连接字符串
    #[serde(rename = "DSN", default)]
    pub dsn: String,

    /// 最大打开连接数（默认 100）
    #[serde(rename = "MaxOpenConns", default = "default_max_open_conns")]
    pub max_open_conns: u32,

    /// 最大空闲连接数（默认 10）
    #[serde(rename = "MaxIdleConns", default = "default_max_idle_conns")]
    pub max_idle_conns: u32,

    /// 连接最大生存时间（duration 字符串，默认 "1h"）
    #[serde(rename = "MaxLifetime", default = "default_max_lifetime")]
    pub max_lifetime: String,

    /// 空闲连接最大存活时间（duration 字符串，默认 "10m"）
    #[serde(rename = "MaxIdleTime", default = "default_max_idle_time")]
    pub max_idle_time: String,

    /// 慢查询阈值（duration 字符串，默认 "200ms"）
    #[serde(rename = "SlowThreshold", default = "default_slow_threshold")]
    pub slow_threshold: String,

    /// 是否启用 SQL 日志（默认 true）
    #[serde(rename = "EnableLog", default = "default_enable_log")]
    pub enable_log: bool,

    /// 实例名称（多实例模式标识，默认空）
    #[serde(rename = "Name", default)]
    pub name: String,
}

fn default_max_open_conns() -> u32 {
    100
}
fn default_max_idle_conns() -> u32 {
    10
}
fn default_max_lifetime() -> String {
    "1h".to_string()
}
fn default_max_idle_time() -> String {
    "10m".to_string()
}
fn default_slow_threshold() -> String {
    "200ms".to_string()
}
fn default_enable_log() -> bool {
    true
}

/// 加载 XOrm 配置（支持单实例和多实例模式）
pub(crate) fn load_configs() -> Vec<XOrmConfig> {
    crate::xconfig::parse_config_list::<XOrmConfig>(XORM_CONFIG_KEY)
}

impl Default for XOrmConfig {
    fn default() -> Self {
        Self {
            driver: Driver::default(),
            dsn: String::new(),
            max_open_conns: default_max_open_conns(),
            max_idle_conns: default_max_idle_conns(),
            max_lifetime: default_max_lifetime(),
            max_idle_time: default_max_idle_time(),
            slow_threshold: default_slow_threshold(),
            enable_log: default_enable_log(),
            name: String::new(),
        }
    }
}
