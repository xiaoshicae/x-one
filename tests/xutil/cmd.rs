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

#[test]
fn test_get_config_from_args_empty_key_returns_none() {
    assert_eq!(get_config_from_args(""), None);
}

#[test]
fn test_get_config_from_args_special_char_key_returns_none() {
    // key 以特殊字符开头，不合法
    assert_eq!(get_config_from_args("@invalid"), None);
    assert_eq!(get_config_from_args("-flag"), None);
}

#[test]
fn test_get_config_from_args_valid_key_with_dots() {
    // 合法 key 但不在实际命令行参数中
    assert_eq!(get_config_from_args("config.file"), None);
}

#[test]
fn test_get_config_from_args_underscore_key() {
    // 下划线开头的合法 key
    assert_eq!(get_config_from_args("_internal"), None);
}
