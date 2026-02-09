//! 网络工具

use local_ip_address::local_ip;
use std::net::IpAddr;

/// 获取本机 IP 地址，优先返回内网 IP
///
/// # Errors
///
/// 无法获取本机 IP 时返回错误
///
/// # Examples
///
/// ```
/// let ip = x_one::xutil::get_local_ip().unwrap();
/// assert!(!ip.is_empty());
/// ```
pub fn get_local_ip() -> Result<String, String> {
    match local_ip() {
        Ok(ip) => Ok(ip.to_string()),
        Err(e) => Err(format!("failed to get local ip: {e}")),
    }
}

/// 提取真实 IP 地址
///
/// 如果 `addr` 是 `0.0.0.0`、`[::]` 或 `::` 则获取本机 IP；
/// 否则解析并验证给定的地址。
///
/// # Errors
///
/// IP 地址无效或无法获取时返回错误
///
/// # Examples
///
/// ```
/// let ip = x_one::xutil::extract_real_ip("192.168.1.1").unwrap();
/// assert_eq!(ip, "192.168.1.1");
/// ```
pub fn extract_real_ip(addr: &str) -> Result<String, String> {
    if !addr.is_empty() && !matches!(addr, "0.0.0.0" | "[::]" | "::") {
        let candidate = addr.trim();

        // 尝试分离 host:port
        let candidate = if let Some(bracket_end) = candidate.find(']') {
            // IPv6 with port: [::1]:8080
            if candidate.len() > bracket_end + 1 && candidate.as_bytes()[bracket_end + 1] == b':' {
                &candidate[..bracket_end + 1]
            } else {
                candidate
            }
        } else if candidate.contains(':') && candidate.matches(':').count() == 1 {
            // IPv4 with port: 192.168.1.1:8080
            candidate.split(':').next().unwrap_or(candidate)
        } else {
            candidate
        };

        // 去掉 IPv6 括号
        let candidate = candidate.trim_start_matches('[').trim_end_matches(']');

        return validate_ip(candidate, addr);
    }

    get_local_ip()
}

/// 验证并解析 IP 地址
pub fn validate_ip(candidate: &str, original: &str) -> Result<String, String> {
    candidate
        .parse::<IpAddr>()
        .map(|ip| ip.to_string())
        .map_err(|_| format!("ip addr {original} is invalid"))
}
