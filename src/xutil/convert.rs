//! 类型转换工具

use std::time::Duration;

/// 将字符串转换为 `Duration`
///
/// 基于 `humantime` 解析，支持常见时间单位：
/// `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, `d` 及其组合。
///
/// 空字符串返回 `Duration::ZERO`。
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// assert_eq!(x_one::xutil::to_duration("1d").unwrap(), Duration::from_secs(86400));
/// assert_eq!(x_one::xutil::to_duration("2d12h").unwrap(), Duration::from_secs(216000));
/// assert_eq!(x_one::xutil::to_duration("1h30m").unwrap(), Duration::from_secs(5400));
/// ```
pub fn to_duration(s: &str) -> Result<Duration, String> {
    if s.is_empty() {
        return Ok(Duration::ZERO);
    }

    humantime::parse_duration(s)
        .map_err(|e| format!("to_duration parse failed: input=[{s}], err=[{e}]"))
}
