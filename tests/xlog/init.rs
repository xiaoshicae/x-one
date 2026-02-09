use x_one::xlog::LogLevel;

#[test]
fn test_get_config_default() {
    let c = x_one::xlog::client::get_config();
    assert_eq!(c.level, LogLevel::Info);
    assert_eq!(c.name, "app");
    assert_eq!(c.path, "./log");
}
