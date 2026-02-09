//! 配置访问器
//!
//! 提供点分 key 路径访问配置值的通用函数。

use super::config_store;

/// 获取配置值（点分 key 路径访问）
pub fn get_value(key: &str) -> Option<serde_yaml::Value> {
    let store = config_store().read();
    let config = store.as_ref()?;

    let mut current = config;
    for k in key.split('.') {
        match current.get(k) {
            Some(v) => current = v,
            None => return None,
        }
    }
    Some(current.clone())
}

/// 获取字符串配置值
pub fn get_string(key: &str) -> String {
    get_value(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// 获取布尔配置值
pub fn get_bool(key: &str) -> bool {
    get_value(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// 获取整数配置值
pub fn get_int(key: &str) -> i64 {
    get_value(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

/// 获取浮点数配置值
pub fn get_float64(key: &str) -> f64 {
    get_value(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// 获取字符串切片配置值
pub fn get_string_slice(key: &str) -> Vec<String> {
    get_value(key)
        .and_then(|v| {
            v.as_sequence().map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// 检查配置中是否包含指定 key
pub fn contain_key(key: &str) -> bool {
    get_value(key).is_some()
}

/// 解析配置值到指定类型
pub fn parse_config<T: serde::de::DeserializeOwned>(
    key: &str,
) -> Result<T, crate::error::XOneError> {
    let value = get_value(key)
        .ok_or_else(|| crate::error::XOneError::Config(format!("config key [{key}] not found")))?;
    serde_yaml::from_value(value).map_err(|e| {
        crate::error::XOneError::Config(format!("parse config [{key}] failed, err=[{e}]"))
    })
}

/// 解析配置值为单个或列表
pub(crate) fn parse_config_list<T: serde::de::DeserializeOwned>(key: &str) -> Vec<T> {
    match parse_config::<T>(key) {
        Ok(config) => return vec![config],
        Err(crate::error::XOneError::Config(ref msg)) if !msg.contains("not found") => {
            crate::xutil::info_if_enable_debug(&format!(
                "parse config [{key}] as single failed: {msg}"
            ));
        }
        _ => {}
    }
    match parse_config::<Vec<T>>(key) {
        Ok(configs) => return configs,
        Err(crate::error::XOneError::Config(ref msg)) if !msg.contains("not found") => {
            crate::xutil::info_if_enable_debug(&format!(
                "parse config [{key}] as list failed: {msg}"
            ));
        }
        _ => {}
    }
    Vec::new()
}
