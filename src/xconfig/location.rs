//! 配置文件路径探测

use crate::xutil;

/// 配置文件位置命令行参数 key
const CONFIG_LOCATION_ARG_KEY: &str = "server.config.location";
/// 配置文件位置环境变量 key
pub const CONFIG_LOCATION_ENV_KEY: &str = "SERVER_CONFIG_LOCATION";

/// 配置文件搜索路径列表，按优先级排序
const CONFIG_LOCATION_PATHS: &[&str] = &[
    "./application.yml",
    "./application.yaml",
    "./conf/application.yml",
    "./conf/application.yaml",
    "./config/application.yml",
    "./config/application.yaml",
    "./../conf/application.yml",
    "./../conf/application.yaml",
    "./../config/application.yml",
    "./../config/application.yaml",
];

/// 检测配置文件位置
///
/// 优先级：命令行参数 > 环境变量 > 当前目录搜索
pub fn detect_config_location() -> Option<String> {
    if let Some(loc) = get_location_from_arg() {
        xutil::info_if_enable_debug(&format!("XOne detect config location [{loc}] from arg"));
        return Some(loc);
    }

    if let Some(loc) = get_location_from_env() {
        xutil::info_if_enable_debug(&format!("XOne detect config location [{loc}] from env"));
        return Some(loc);
    }

    if let Some(loc) = get_location_from_current_dir() {
        xutil::info_if_enable_debug(&format!(
            "XOne detect config location [{loc}] from current dir"
        ));
        return Some(loc);
    }

    None
}

fn get_location_from_arg() -> Option<String> {
    xutil::get_config_from_args(CONFIG_LOCATION_ARG_KEY).ok()
}

pub fn get_location_from_env() -> Option<String> {
    std::env::var(CONFIG_LOCATION_ENV_KEY)
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn get_location_from_current_dir() -> Option<String> {
    CONFIG_LOCATION_PATHS
        .iter()
        .find(|loc| xutil::file_exist(loc))
        .map(|loc| loc.to_string())
}
