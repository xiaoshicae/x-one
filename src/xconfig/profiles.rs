//! 多环境 profile 配置支持

use crate::xutil;

/// Profile active 命令行参数 key
const PROFILES_ACTIVE_ARG_KEY: &str = "server.profiles.active";
/// Profile active 环境变量 key
pub const PROFILES_ACTIVE_ENV_KEY: &str = "SERVER_PROFILES_ACTIVE";
/// Profile active 配置文件中的 key 路径
const PROFILES_ACTIVE_CONFIG_KEY: &str = "Server.Profiles.Active";

/// 检测激活的环境
///
/// 优先级：命令行参数 > 环境变量 > 配置文件中的值
pub fn detect_profiles_active(config: &serde_yaml::Value) -> Option<String> {
    if let Some(pa) = get_profiles_active_from_arg() {
        xutil::info_if_enable_debug(&format!("XOne detect profiles active [{pa}] from arg"));
        return Some(pa);
    }

    if let Some(pa) = get_profiles_active_from_env() {
        xutil::info_if_enable_debug(&format!("XOne detect profiles active [{pa}] from env"));
        return Some(pa);
    }

    if let Some(pa) = get_profiles_active_from_config(config) {
        xutil::info_if_enable_debug(&format!(
            "XOne detect profiles active [{pa}] from base config file"
        ));
        return Some(pa);
    }

    xutil::info_if_enable_debug("XOne config profiles active not found");
    None
}

#[doc(hidden)]
pub fn get_profiles_active_from_env() -> Option<String> {
    std::env::var(PROFILES_ACTIVE_ENV_KEY)
        .ok()
        .filter(|s| !s.is_empty())
}

#[doc(hidden)]
pub fn get_profiles_active_from_config(config: &serde_yaml::Value) -> Option<String> {
    // 按照 "Server.Profiles.Active" 的点分路径访问
    let keys: Vec<&str> = PROFILES_ACTIVE_CONFIG_KEY.split('.').collect();
    let mut current = config;
    for key in keys {
        match current.get(key) {
            Some(v) => current = v,
            None => return None,
        }
    }
    current
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// 根据基础配置文件路径和激活的环境，构建环境配置文件路径
///
/// 例如: `./conf/application.yml` + `dev` -> `./conf/application-dev.yml`
pub fn to_profiles_active_config_location(
    config_location: &str,
    profile_active: &str,
) -> Result<String, crate::error::XOneError> {
    let path = std::path::Path::new(config_location);
    let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        crate::error::XOneError::Config(
            "config file name is invalid, no extension found".to_string(),
        )
    })?;

    let stem = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        crate::error::XOneError::Config("config file name is invalid, no stem found".to_string())
    })?;

    let file_name = format!("{stem}-{profile_active}.{ext}");
    let result = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""))
        .join(&file_name);
    Ok(result.to_string_lossy().to_string())
}

fn get_profiles_active_from_arg() -> Option<String> {
    xutil::get_config_from_args(PROFILES_ACTIVE_ARG_KEY).filter(|s| !s.is_empty())
}
