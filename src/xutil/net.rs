//! 网络工具函数
//!
//! 获取本机 IP 地址，支持按公网/内网优先级筛选。

use std::net::IpAddr;

/// IP 地址分类结果
struct IpGroups {
    pub4: Vec<IpAddr>,
    pub6: Vec<IpAddr>,
    pri4: Vec<IpAddr>,
    pri6: Vec<IpAddr>,
}

/// 获取本机 IP 地址
///
/// 优先级：public IPv4 → public IPv6 → private IPv4 → private IPv6
///
/// # Examples
///
/// ```
/// let ip = x_one::xutil::get_local_ip();
/// // ip 可能为 Ok("192.168.1.100") 或 Err(...)
/// ```
pub fn get_local_ip() -> Result<String, String> {
    let groups = collect_local_ips();
    if let Some(ip) = groups.pub4.first() {
        return Ok(ip.to_string());
    }
    if let Some(ip) = groups.pub6.first() {
        return Ok(ip.to_string());
    }
    if let Some(ip) = groups.pri4.first() {
        return Ok(ip.to_string());
    }
    if let Some(ip) = groups.pri6.first() {
        return Ok(ip.to_string());
    }
    Err("no IP address found".to_string())
}

/// 获取本机公网 IP
///
/// 优先级：public IPv4 → public IPv6
pub fn get_local_public_ip() -> Result<String, String> {
    let groups = collect_local_ips();
    if let Some(ip) = groups.pub4.first() {
        return Ok(ip.to_string());
    }
    if let Some(ip) = groups.pub6.first() {
        return Ok(ip.to_string());
    }
    Err("no public IP address found".to_string())
}

/// 获取本机内网 IP
///
/// 优先级：private IPv4 → private IPv6
pub fn get_local_private_ip() -> Result<String, String> {
    let groups = collect_local_ips();
    if let Some(ip) = groups.pri4.first() {
        return Ok(ip.to_string());
    }
    if let Some(ip) = groups.pri6.first() {
        return Ok(ip.to_string());
    }
    Err("no private IP address found".to_string())
}

/// 判断 IP 是否为私有地址
fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// 收集本机所有 IP，按类型分类
fn collect_local_ips() -> IpGroups {
    let mut groups = IpGroups {
        pub4: Vec::new(),
        pub6: Vec::new(),
        pri4: Vec::new(),
        pri6: Vec::new(),
    };

    let interfaces = pnet_datalink::interfaces();
    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }
        for ip_net in &iface.ips {
            let ip = ip_net.ip();
            if ip.is_loopback() {
                continue;
            }
            match ip {
                IpAddr::V4(_) => {
                    if is_private(&ip) {
                        groups.pri4.push(ip);
                    } else {
                        groups.pub4.push(ip);
                    }
                }
                IpAddr::V6(_) => {
                    if is_private(&ip) {
                        groups.pri6.push(ip);
                    } else {
                        groups.pub6.push(ip);
                    }
                }
            }
        }
    }

    groups
}
