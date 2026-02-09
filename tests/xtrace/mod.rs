use x_one::xtrace::*;

#[test]
fn test_register_hook_idempotent() {
    // 多次调用不 panic
    register_hook();
    register_hook();
}

#[test]
fn test_is_trace_enabled_api() {
    // 调用不 panic
    let _ = is_trace_enabled();
}
