use serial_test::serial;
use x_one::xutil::env::*;

/// 辅助函数：安全地设置环境变量（测试专用）
fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

/// 辅助函数：安全地移除环境变量（测试专用）
fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) };
}

#[test]
#[serial]
fn test_xone_enable_debug_true() {
    set_env(DEBUG_KEY, "true");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_1() {
    set_env(DEBUG_KEY, "1");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_yes() {
    set_env(DEBUG_KEY, "yes");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_on() {
    set_env(DEBUG_KEY, "on");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_t() {
    set_env(DEBUG_KEY, "T");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_y() {
    set_env(DEBUG_KEY, "Y");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_false() {
    set_env(DEBUG_KEY, "false");
    assert!(!xone_enable_debug());
    remove_env(DEBUG_KEY);
}

#[test]
#[serial]
fn test_xone_enable_debug_empty() {
    remove_env(DEBUG_KEY);
    assert!(!xone_enable_debug());
}

#[test]
#[serial]
fn test_xone_enable_debug_with_whitespace() {
    set_env(DEBUG_KEY, "  true  ");
    assert!(xone_enable_debug());
    remove_env(DEBUG_KEY);
}
