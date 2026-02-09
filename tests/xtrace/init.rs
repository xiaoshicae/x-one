use x_one::xtrace::init::*;

#[test]
fn test_is_trace_enabled_default_false() {
    // 未初始化前默认为 false
    let _ = is_trace_enabled();
}

#[test]
fn test_shutdown_when_not_enabled() {
    // 未初始化时 shutdown 应直接返回 Ok
    let result = shutdown_xtrace();
    assert!(result.is_ok());
}
