//! 默认值工具
//!
//! 提供「值为空时返回 fallback」的泛型工具函数。

/// 判断值是否为空
pub trait IsEmpty {
    /// 值是否为"空"
    fn is_empty(&self) -> bool;
}

impl IsEmpty for str {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl IsEmpty for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> IsEmpty for Option<T> {
    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

/// 若值为空则返回 fallback（借用版本）
///
/// # Examples
///
/// ```
/// assert_eq!(x_one::xutil::default_if_empty("", "fallback"), "fallback");
/// assert_eq!(x_one::xutil::default_if_empty("hello", "fallback"), "hello");
/// ```
pub fn default_if_empty<'a, T: IsEmpty + ?Sized>(value: &'a T, fallback: &'a T) -> &'a T {
    if value.is_empty() { fallback } else { value }
}

/// 若值为空则返回 fallback（所有权版本）
///
/// fallback 支持自动类型转换（如 `&str` → `String`）。
///
/// # Examples
///
/// ```
/// let result = x_one::xutil::take_or_default(String::new(), "default");
/// assert_eq!(result, "default");
/// ```
pub fn take_or_default<T: IsEmpty, F: Into<T>>(value: T, fallback: F) -> T {
    if value.is_empty() { fallback.into() } else { value }
}
