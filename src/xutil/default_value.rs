//! 默认值工具
//!
//! 提供「值为零值时返回 fallback」的泛型工具函数。
//!
//! 零值判定规则：
//! - `String` / 数值 / `Vec` / `Option` 等实现了 `Default + PartialEq` 的类型：等于 `T::default()` 即为零值
//! - `str`：空字符串即为零值

/// 判断值是否为零值
pub trait IsZero {
    /// 值是否为"零值"
    fn is_zero(&self) -> bool;
}

/// 所有实现了 `Default + PartialEq` 的类型自动获得 `IsZero`
///
/// 覆盖：`String`(`""`)、`Vec`(`[]`)、`Option`(`None`)、
/// 数值类型(`0`)、`bool`(`false`) 等。
impl<T: Default + PartialEq> IsZero for T {
    fn is_zero(&self) -> bool {
        *self == T::default()
    }
}

/// `str` 是 unsized 类型，无法实现 `Default`，需单独处理
impl IsZero for str {
    fn is_zero(&self) -> bool {
        self.is_empty()
    }
}

/// 若值为零值则返回 fallback（借用版本）
///
/// # Examples
///
/// ```
/// assert_eq!(x_one::xutil::default_if_empty("", "fallback"), "fallback");
/// assert_eq!(x_one::xutil::default_if_empty("hello", "fallback"), "hello");
/// assert_eq!(x_one::xutil::default_if_empty(&0_i32, &42), &42);
/// ```
pub fn default_if_empty<'a, T: IsZero + ?Sized>(value: &'a T, fallback: &'a T) -> &'a T {
    if value.is_zero() { fallback } else { value }
}

/// 若值为零值则返回 fallback（所有权版本）
///
/// fallback 支持自动类型转换（如 `&str` → `String`）。
///
/// # Examples
///
/// ```
/// let result = x_one::xutil::take_or_default(String::new(), "default");
/// assert_eq!(result, "default");
///
/// assert_eq!(x_one::xutil::take_or_default(0_u64, 100_u64), 100);
/// ```
pub fn take_or_default<T: IsZero, F: Into<T>>(value: T, fallback: F) -> T {
    if value.is_zero() { fallback.into() } else { value }
}
