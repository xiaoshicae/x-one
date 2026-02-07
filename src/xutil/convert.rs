//! 类型转换工具

use std::time::Duration;

use super::debug_log;

/// 将字符串转换为 `Duration`，支持天数格式
///
/// 支持的格式：
/// - 标准 Go 风格: `"1h30m"`, `"100ms"`, `"2s"`
/// - 扩展天数格式: `"1d"`, `"2d12h"`, `"7d"`
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// assert_eq!(x_one::xutil::to_duration("1d").unwrap(), Duration::from_secs(86400));
/// assert_eq!(x_one::xutil::to_duration("2d12h").unwrap(), Duration::from_secs(60 * 3600));
/// assert_eq!(x_one::xutil::to_duration("1h30m").unwrap(), Duration::from_secs(5400));
/// ```
pub fn to_duration(s: &str) -> Result<Duration, String> {
    if s.is_empty() {
        return Ok(Duration::ZERO);
    }

    // 如果包含 'd'，拆分天数部分和剩余部分
    if let Some(d_pos) = s.find('d') {
        let day_part = &s[..d_pos];
        let rest = &s[d_pos + 1..];

        match day_part.parse::<u64>() {
            Ok(days) => {
                let day_duration = Duration::from_secs(days * 24 * 3600);
                if rest.is_empty() {
                    return Ok(day_duration);
                }
                return parse_go_duration(rest).map(|rest_duration| day_duration + rest_duration);
            }
            Err(e) => {
                let msg = format!(
                    "to_duration parse day failed, day=[{day_part}], err=[{e}], fallback to parse left=[{rest}]"
                );
                debug_log::error_if_enable_debug(&msg);
                if rest.is_empty() {
                    return Err(msg);
                }
                return parse_go_duration(rest);
            }
        }
    }

    parse_go_duration(s)
}

/// 解析 Go 风格的 duration 字符串（不含天数）
///
/// 支持: `h`(小时), `m`(分钟), `s`(秒), `ms`(毫秒), `us`/`µs`(微秒), `ns`(纳秒)
fn parse_go_duration(s: &str) -> Result<Duration, String> {
    if s.is_empty() {
        return Ok(Duration::ZERO);
    }

    let mut total = Duration::ZERO;
    let mut remaining = s;

    while !remaining.is_empty() {
        // 跳过前导空白
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        // 解析数字部分
        let num_end = remaining
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(remaining.len());

        if num_end == 0 {
            let msg = format!("parse_go_duration: unexpected character in [{s}]");
            debug_log::error_if_enable_debug(&msg);
            return Err(msg);
        }

        let num_str = &remaining[..num_end];
        remaining = &remaining[num_end..];

        let value: f64 = match num_str.parse() {
            Ok(v) => v,
            Err(_) => {
                let msg = format!("parse_go_duration: invalid number [{num_str}] in [{s}]");
                debug_log::error_if_enable_debug(&msg);
                return Err(msg);
            }
        };

        // 解析单位部分
        let unit_end = remaining
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(remaining.len());

        let unit = &remaining[..unit_end];
        remaining = &remaining[unit_end..];

        let multiplier: f64 = match unit {
            "ns" => 1e-9,
            "us" | "µs" => 1e-6,
            "ms" => 1e-3,
            "s" | "" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            _ => {
                let msg = format!("parse_go_duration: unknown unit [{unit}] in [{s}]");
                debug_log::error_if_enable_debug(&msg);
                return Err(msg);
            }
        };

        total += Duration::from_secs_f64(value * multiplier);
    }

    Ok(total)
}


