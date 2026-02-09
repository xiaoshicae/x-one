//! 命令行参数解析工具

use regex::Regex;
use std::sync::LazyLock;

/// 合法的参数 key 正则
static ARG_KEY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_.\-]*$").expect("硬编码正则表达式无效"));

/// 从启动命令行参数中获取指定 key 的值
///
/// 仅识别 `--` 双破折号前缀，支持两种格式：
/// - 空格分隔：`--key value`
/// - 等号分隔：`--key=value`
///
/// key 不合法或找不到对应参数时返回 `None`。
///
/// # Examples
///
/// ```
/// let args = vec![
///     "--config".to_string(),
///     "app.yml".to_string(),
/// ];
/// let result = x_one::xutil::get_config_from_args_with("config", &args);
/// assert_eq!(result, Some("app.yml".to_string()));
/// ```
pub fn get_config_from_args(key: &str) -> Option<String> {
    let args = get_os_args();
    get_config_from_args_with(key, &args)
}

/// 使用给定的参数列表获取指定 key 的值（方便测试）
pub fn get_config_from_args_with(key: &str, args: &[String]) -> Option<String> {
    if !ARG_KEY_PATTERN.is_match(key) {
        return None;
    }

    let eq_suffix = format!("{key}=");

    for (i, arg) in args.iter().enumerate() {
        let Some(after_dash) = arg.strip_prefix("--") else {
            continue;
        };

        // 空格分隔：--config value
        if after_dash == key {
            return args.get(i + 1).cloned();
        }

        // 等号分隔：--config=value
        if let Some(value) = after_dash.strip_prefix(&eq_suffix) {
            return Some(value.to_string());
        }
    }

    None
}

/// 获取进程启动参数（跳过第一个可执行文件路径）
pub fn get_os_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}
