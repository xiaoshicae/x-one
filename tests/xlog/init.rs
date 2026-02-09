use x_one::xlog::{LogLevel, XLogConfig};

#[test]
fn test_xlog_config_default() {
    let c = XLogConfig::default();
    assert_eq!(c.level, LogLevel::Info);
    assert_eq!(c.name, "app");
    assert_eq!(c.path, "./log");
}
