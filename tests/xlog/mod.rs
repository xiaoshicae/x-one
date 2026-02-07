use x_one::xlog::xlog_level;

#[test]
fn test_xlog_level_default() {
    assert_eq!(xlog_level(), "info");
}

#[test]
fn test_xlog_macros_no_panic() {
    // 宏调用不应 panic（即使 subscriber 未初始化也不应 panic）
    x_one::xlog_info!("test info message");
    x_one::xlog_error!("test error message");
    x_one::xlog_warn!("test warn message");
    x_one::xlog_debug!("test debug message");
}
