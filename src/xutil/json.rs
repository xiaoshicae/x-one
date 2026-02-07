//! JSON 序列化工具

use super::debug_log;

/// 将值转换为 JSON 字符串
///
/// 序列化失败时返回空字符串，并在 debug 模式下记录错误日志。
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// let mut m = HashMap::new();
/// m.insert("key", "value");
/// let result = x_one::xutil::to_json_string(&m);
/// assert!(result.contains("key"));
/// assert!(result.contains("value"));
/// ```
pub fn to_json_string<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_string(v) {
        Ok(s) => s,
        Err(e) => {
            debug_log::error_if_enable_debug(&format!("to_json_string failed, err=[{e}]"));
            String::new()
        }
    }
}

/// 将值转换为格式化的 JSON 字符串（带缩进）
///
/// 序列化失败时返回空字符串，并在 debug 模式下记录错误日志。
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// let mut m = HashMap::new();
/// m.insert("key", "value");
/// let result = x_one::xutil::to_json_string_indent(&m);
/// assert!(result.contains("key"));
/// assert!(result.contains("value"));
/// assert!(result.contains('\n')); // 格式化输出包含换行
/// ```
pub fn to_json_string_indent<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_string_pretty(v) {
        Ok(s) => s,
        Err(e) => {
            debug_log::error_if_enable_debug(&format!("to_json_string_indent failed, err=[{e}]"));
            String::new()
        }
    }
}


