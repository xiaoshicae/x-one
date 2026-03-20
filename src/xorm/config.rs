//! xorm 配置结构体
//!
//! 对应 `application.yml` 中的 `XOrm` 节点，支持单实例和多实例配置。
//!
//! ```yaml
//! # 单实例
//! XOrm:
//!   Driver: "postgres"
//!   DSN: "postgres://user:pass@localhost/db"
//!
//! # 多实例
//! XOrm:
//!   - Driver: "postgres"
//!     DSN: "postgres://..."
//!     Name: "primary"
//!   - Driver: "mysql"
//!     DSN: "mysql://..."
//!     Name: "analytics"
//! ```

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
#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct XOrmConfig {
    /// 数据库驱动（默认 postgres）
    #[serde(rename = "Driver")]
    pub driver: Driver,

    /// 数据库连接字符串
    #[serde(rename = "DSN")]
    pub dsn: String,

    /// 最大打开连接数（默认 100）
    #[serde(rename = "MaxOpenConns")]
    pub max_open_conns: u32,

    /// 最大空闲连接数（默认 10）
    #[serde(rename = "MaxIdleConns")]
    pub max_idle_conns: u32,

    /// 连接最大生存时间（duration 字符串，默认 "1h"）
    #[serde(rename = "MaxLifetime")]
    pub max_lifetime: String,

    /// 空闲连接最大存活时间（duration 字符串，默认 "10m"）
    #[serde(rename = "MaxIdleTime")]
    pub max_idle_time: String,

    /// 慢查询阈值（duration 字符串，默认 "200ms"）
    #[serde(rename = "SlowThreshold")]
    pub slow_threshold: String,

    /// 是否启用 SQL 日志（默认 true）
    #[serde(rename = "EnableLog")]
    pub enable_log: bool,

    /// 实例名称（多实例模式标识，默认空）
    #[serde(rename = "Name")]
    pub name: String,
}

/// 手动实现 Debug，DSN 中的密码部分脱敏
impl std::fmt::Debug for XOrmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XOrmConfig")
            .field("driver", &self.driver)
            .field("dsn", &sanitize_dsn(&self.dsn))
            .field("max_open_conns", &self.max_open_conns)
            .field("max_idle_conns", &self.max_idle_conns)
            .field("max_lifetime", &self.max_lifetime)
            .field("max_idle_time", &self.max_idle_time)
            .field("slow_threshold", &self.slow_threshold)
            .field("enable_log", &self.enable_log)
            .field("name", &self.name)
            .finish()
    }
}

/// 脱敏 DSN 中的密码部分
///
/// 将 `postgres://user:pass@host/db` 替换为 `postgres://user:***@host/db`
fn sanitize_dsn(dsn: &str) -> String {
    // 匹配 scheme://user:password@host 格式
    if let Some(scheme_end) = dsn.find("://") {
        let after_scheme = &dsn[scheme_end + 3..];
        if let Some(at_pos) = after_scheme.find('@') {
            let userinfo = &after_scheme[..at_pos];
            if let Some(colon_pos) = userinfo.find(':') {
                let user = &userinfo[..colon_pos];
                let after_at = &after_scheme[at_pos..];
                return format!("{}://{}:***{}", &dsn[..scheme_end], user, after_at);
            }
        }
    }
    dsn.to_string()
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
            max_open_conns: 100,
            max_idle_conns: 10,
            max_lifetime: "1h".to_string(),
            max_idle_time: "10m".to_string(),
            slow_threshold: "200ms".to_string(),
            enable_log: true,
            name: String::new(),
        }
    }
}
