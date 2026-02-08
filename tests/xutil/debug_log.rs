use serial_test::serial;
use x_one::xutil::debug_log::*;

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) };
}

#[test]
#[serial]
fn test_info_if_enable_debug_enabled() {
    set_env(x_one::xutil::env::DEBUG_KEY, "true");
    info_if_enable_debug("test message");
    remove_env(x_one::xutil::env::DEBUG_KEY);
}

#[test]
#[serial]
fn test_warn_if_enable_debug_enabled() {
    set_env(x_one::xutil::env::DEBUG_KEY, "true");
    warn_if_enable_debug("test warning");
    remove_env(x_one::xutil::env::DEBUG_KEY);
}

#[test]
#[serial]
fn test_error_if_enable_debug_enabled() {
    set_env(x_one::xutil::env::DEBUG_KEY, "true");
    error_if_enable_debug("test error");
    remove_env(x_one::xutil::env::DEBUG_KEY);
}

#[test]
#[serial]
fn test_log_if_enable_debug_disabled() {
    remove_env(x_one::xutil::env::DEBUG_KEY);
    info_if_enable_debug("should not print");
}
