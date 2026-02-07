use std::time::Duration;
use x_one::xtrace::*;

#[test]
fn test_set_shutdown_timeout() {
    set_shutdown_timeout(Duration::from_secs(10));
    assert_eq!(get_shutdown_timeout(), Duration::from_secs(10));
    // 恢复默认值
    set_shutdown_timeout(Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS));
}

#[test]
fn test_set_shutdown_timeout_zero_ignored() {
    let before = get_shutdown_timeout();
    set_shutdown_timeout(Duration::ZERO);
    assert_eq!(get_shutdown_timeout(), before);
}

#[test]
fn test_register_hook_idempotent() {
    // 多次调用不 panic
    register_hook();
    register_hook();
}

#[test]
fn test_enable_trace_api() {
    // 调用不 panic
    let _ = enable_trace();
}

#[test]
fn test_get_tracer_api() {
    let tracer = get_tracer("test");
    let _ = tracer;
}
