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
    serialize_or_empty(serde_json::to_string(v), "to_json_string")
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
    serialize_or_empty(serde_json::to_string_pretty(v), "to_json_string_indent")
}

/// 序列化结果处理：成功返回字符串，失败记录日志并返回空字符串
fn serialize_or_empty(result: serde_json::Result<String>, label: &str) -> String {
    match result {
        Ok(s) => s,
        Err(e) => {
            debug_log::error_if_enable_debug(&format!("{label} failed, err=[{e}]"));
            String::new()
        }
    }
}
