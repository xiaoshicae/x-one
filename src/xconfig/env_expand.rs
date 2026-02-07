//! 环境变量占位符展开
//!
//! 支持 `${VAR}` 和 `${VAR:-default}` 格式

use regex::Regex;
use std::sync::LazyLock;

/// 环境变量占位符正则
static ENV_PLACEHOLDER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}").unwrap());

/// 展开字符串中的环境变量占位符
///
/// 支持 `${VAR}` 和 `${VAR:-default}` 格式
pub fn expand_env_placeholder(val: &str) -> String {
    ENV_PLACEHOLDER_REGEX
        .replace_all(val, |caps: &regex::Captures| {
            let env_key = &caps[1];
            let default_val = caps.get(2).map_or("", |m| m.as_str());

            match std::env::var(env_key) {
                Ok(env_val) if !env_val.is_empty() => env_val,
                _ => default_val.to_string(),
            }
        })
        .to_string()
}

/// 展开 YAML Value 中所有字符串的环境变量占位符
pub fn expand_env_placeholders_in_value(value: &mut serde_yaml::Value) {
    match value {
        serde_yaml::Value::String(s) => {
            let expanded = expand_env_placeholder(s);
            if expanded != *s {
                *s = expanded;
            }
        }
        serde_yaml::Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                expand_env_placeholders_in_value(v);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for v in seq.iter_mut() {
                expand_env_placeholders_in_value(v);
            }
        }
        _ => {}
    }
}


