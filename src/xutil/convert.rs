//! 类型转换工具

use std::time::Duration;

/// 将字符串转换为 `Duration`
///
/// 基于 `humantime` 解析，支持常见时间单位：
/// `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, `d` 及其组合。
///
/// 空字符串或纯空白返回 `None`（调用方应使用默认值），解析失败也返回 `None`。
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// assert_eq!(x_one::xutil::to_duration("1d"), Some(Duration::from_secs(86400)));
/// assert_eq!(x_one::xutil::to_duration("2d12h"), Some(Duration::from_secs(216000)));
/// assert_eq!(x_one::xutil::to_duration("1h30m"), Some(Duration::from_secs(5400)));
/// assert_eq!(x_one::xutil::to_duration(""), None);
/// ```
pub fn to_duration(s: &str) -> Option<Duration> {
    if s.trim().is_empty() {
        return None;
    }

    humantime::parse_duration(s).ok()
}
