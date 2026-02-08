//! 命令行参数解析工具

use regex::Regex;
use std::sync::LazyLock;

/// 合法的参数 key 正则
static ARG_KEY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_.\-]*$").unwrap());

/// 从启动命令行参数中获取指定 key 的值
///
/// 支持两种格式：
/// - 空格分隔：`--key value`
/// - 等号分隔：`--key=value`
///
/// # Errors
///
/// - key 不匹配正则时返回错误
/// - 找不到对应参数时返回错误
///
/// # Examples
///
/// ```
/// // 使用自定义参数列表进行测试
/// let args = vec![
///     "--config".to_string(),
///     "app.yml".to_string(),
/// ];
/// let result = x_one::xutil::get_config_from_args_with("config", &args);
/// assert_eq!(result.unwrap(), "app.yml");
/// ```
pub fn get_config_from_args(key: &str) -> Result<String, String> {
    let args = get_os_args();
    get_config_from_args_with(key, &args)
}

/// 使用给定的参数列表获取指定 key 的值（方便测试）
pub fn get_config_from_args_with(key: &str, args: &[String]) -> Result<String, String> {
    if !ARG_KEY_PATTERN.is_match(key) {
        return Err(format!(
            "key must match regexp: {}",
            ARG_KEY_PATTERN.as_str()
        ));
    }

    if args.is_empty() {
        return Err("arg not found, there is no arg".to_string());
    }

    for (i, arg) in args.iter().enumerate() {
        let trimmed = arg.trim_start_matches('-');

        // 空格配置方式 --config value
        if trimmed == key {
            if i + 1 >= args.len() {
                return Err("arg not found, arg not set".to_string());
            }
            return Ok(args[i + 1].clone());
        }

        // 等号配置方式 --config=value
        let prefix = format!("{key}=");
        if trimmed.starts_with(&prefix) {
            return Ok(trimmed[prefix.len()..].to_string());
        }
    }

    Err("arg not found".to_string())
}

/// 获取进程启动参数（跳过第一个可执行文件路径）
pub fn get_os_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}
