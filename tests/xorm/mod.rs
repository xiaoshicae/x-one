use x_one::xorm::*;

#[test]
fn test_register_hook_idempotent() {
    register_hook();
    register_hook();
}

#[test]
fn test_get_pool_config_api() {
    let config = get_pool_config(None);
    // 没有初始化时返回 None
    let _ = config;
}

#[test]
fn test_get_pool_names_api() {
    let names = get_pool_names();
    let _ = names;
}
