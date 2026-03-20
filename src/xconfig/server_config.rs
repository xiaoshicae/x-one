//! Server 相关配置获取
//!
//! 提供服务名、版本等 Server 级别的配置访问。

use super::accessor::get_string;

/// 默认服务名
pub const DEFAULT_SERVER_NAME: &str = "unknown.unknown.unknown";
/// 默认服务版本
pub const DEFAULT_SERVER_VERSION: &str = "v0.0.1";

/// Server.Name 配置路径
const SERVER_NAME_KEY: &str = "Server.Name";
/// Server.Version 配置路径
const SERVER_VERSION_KEY: &str = "Server.Version";

/// 获取服务名（未配置时返回默认值）
pub fn get_server_name() -> String {
    crate::xutil::take_or_default(get_string(SERVER_NAME_KEY), DEFAULT_SERVER_NAME)
}

/// 获取原始服务名（未配置时返回空字符串）
pub fn get_raw_server_name() -> String {
    get_string(SERVER_NAME_KEY)
}

/// 获取服务版本（未配置时返回默认值）
pub fn get_server_version() -> String {
    crate::xutil::take_or_default(get_string(SERVER_VERSION_KEY), DEFAULT_SERVER_VERSION)
}
