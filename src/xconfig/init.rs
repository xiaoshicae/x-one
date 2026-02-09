//! 配置初始化

use super::env_expand;
use super::location;
use super::profiles;
use crate::xutil;
use std::path::Path;

/// .env 文件名
const DOT_ENV_FILENAME: &str = ".env";

/// 初始化配置系统
///
/// 检测配置文件位置、加载 .env 文件、解析配置
pub fn init_xconfig() -> Result<Option<serde_yaml::Value>, String> {
    let config_location = match location::detect_config_location() {
        Some(loc) => loc,
        None => {
            xutil::warn_if_enable_debug(
                "XOne initXConfig config file location not found, use default config",
            );
            return Ok(None);
        }
    };

    load_dot_env_if_exist(&config_location)?;

    let config = parse_config(&config_location)?;

    print_final_config(&config);

    Ok(Some(config))
}

/// 加载 .env 文件（如果存在）
fn load_dot_env_if_exist(config_location: &str) -> Result<(), String> {
    let dir = Path::new(config_location)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let dot_env_path = if dir.is_empty() {
        DOT_ENV_FILENAME.to_string()
    } else {
        format!("{dir}/{DOT_ENV_FILENAME}")
    };

    if xutil::file_exist(&dot_env_path) {
        dotenvy::from_filename(&dot_env_path)
            .map_err(|e| format!("XOne initXConfig load .env failed, err=[{e}]"))?;
    }
    Ok(())
}

/// 解析配置文件
fn parse_config(config_location: &str) -> Result<serde_yaml::Value, String> {
    // 加载基础配置
    let mut base_config = load_local_config(config_location)?;

    // 先展开基础配置中的环境变量占位符，使 profiles.active 可引用环境变量
    env_expand::expand_env_placeholders_in_value(&mut base_config);

    // 检测激活的环境
    if let Some(pa) = profiles::detect_profiles_active(&base_config) {
        let env_config_location =
            profiles::to_profiles_active_config_location(config_location, &pa)?;

        if !xutil::file_exist(&env_config_location) {
            xutil::warn_if_enable_debug(&format!(
                "XOne profiles active config file not found, ignore, env_config_location=[{env_config_location}]"
            ));
        } else {
            let env_config = load_local_config(&env_config_location)?;
            base_config = merge_profiles_config(base_config, env_config);
        }
    }

    // 检查 Server.Name 是否为空
    if let Some(name) = base_config
        .get("Server")
        .and_then(|s| s.get("Name"))
        .and_then(|n| n.as_str())
    {
        if name.is_empty() {
            xutil::warn_if_enable_debug(
                "config Server.Name should not be empty, as it is used by many modules",
            );
        }
    } else {
        xutil::warn_if_enable_debug(
            "config Server.Name should not be empty, as it is used by many modules",
        );
    }

    // 合并 profile 配置后再次展开，处理 profile 配置中的环境变量占位符
    env_expand::expand_env_placeholders_in_value(&mut base_config);

    Ok(base_config)
}

/// 加载本地配置文件
pub fn load_local_config(path: &str) -> Result<serde_yaml::Value, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("load config file failed, err=[{e}]"))?;
    serde_yaml::from_str(&content)
        .map_err(|e| format!("parse config file failed, path=[{path}], err=[{e}]"))
}

/// 合并环境配置到基础配置
///
/// 环境配置中的顶层 key 会覆盖基础配置中的对应 key，
/// Server 下的二级 key 也会单独覆盖
pub fn merge_profiles_config(
    mut base: serde_yaml::Value,
    env: serde_yaml::Value,
) -> serde_yaml::Value {
    if let (Some(base_map), Some(env_map)) = (base.as_mapping_mut(), env.as_mapping()) {
        for (key, value) in env_map {
            // 跳过 Server.Profiles
            if let Some("Server") = key.as_str() {
                // Server 下进行二级 key 合并
                if let (Some(base_server), Some(env_server)) = (
                    base_map.get_mut(key).and_then(|v| v.as_mapping_mut()),
                    value.as_mapping(),
                ) {
                    for (sk, sv) in env_server {
                        // 跳过 Profiles
                        if sk.as_str() == Some("Profiles") {
                            continue;
                        }
                        base_server.insert(sk.clone(), sv.clone());
                    }
                }
                continue;
            }
            base_map.insert(key.clone(), value.clone());
        }
    }
    base
}

/// 打印最终配置（debug 模式下）
fn print_final_config(config: &serde_yaml::Value) {
    if xutil::enable_debug() {
        let config_str = xutil::to_json_string_indent(config);
        eprintln!(
            "\n************************************** XOne load config **************************************\n{config_str}\n**********************************************************************************************\n"
        );
    }
}
