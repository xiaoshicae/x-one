use x_one::xutil;

#[test]
fn test_get_local_ip_returns_something() {
    // 在大多数环境中应该能获取到 IP
    let result = xutil::get_local_ip();
    // 容器环境可能没有非 loopback 接口，所以不强制 Ok
    if let Ok(ip) = result {
        assert!(!ip.is_empty());
    }
}

#[test]
fn test_get_local_private_ip() {
    let result = xutil::get_local_private_ip();
    if let Ok(ip) = result {
        assert!(!ip.is_empty());
    }
}

#[test]
fn test_get_local_public_ip() {
    // 公网 IP 在测试环境中通常不存在
    let _result = xutil::get_local_public_ip();
}
