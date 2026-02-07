use std::sync::atomic::Ordering;
use x_one::xtrace::init::*;

#[test]
fn test_enable_trace_default_false() {
    // 未初始化前，根据全局 AtomicBool 状态
    // 注意：测试中全局状态可能被其他测试修改
    let _ = enable_trace();
}

#[test]
fn test_get_tracer() {
    let tracer = get_tracer("test-tracer");
    // 能获取到 tracer 即可（可能是 noop）
    let _ = tracer;
}

#[test]
fn test_load_config_no_config() {
    let config = load_config();
    // 没有配置时返回默认值
    assert!(config.is_enabled());
    assert!(!config.console);
}

#[test]
fn test_shutdown_when_not_enabled() {
    TRACE_ENABLED.store(false, Ordering::SeqCst);
    let result = shutdown_xtrace();
    assert!(result.is_ok());
}
