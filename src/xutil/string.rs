//! 字符串辅助工具

/// 若为空字符串则返回默认值（借用）
pub fn default_if_empty<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.is_empty() { default } else { value }
}

/// 若为空字符串则返回默认值（占有）
pub fn take_or_default(value: String, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}
