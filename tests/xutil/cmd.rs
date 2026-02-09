use x_one::xutil::get_config_from_args;

#[test]
fn test_get_config_from_args_not_found() {
    // 测试进程不带 --nonexistent 参数，应返回 None
    assert_eq!(get_config_from_args("nonexistent"), None);
}

#[test]
fn test_get_config_from_args_invalid_key() {
    assert_eq!(get_config_from_args("123invalid"), None);
}
