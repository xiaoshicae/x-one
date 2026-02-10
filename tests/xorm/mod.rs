use x_one::xorm::*;

#[test]
fn test_register_hook_idempotent() {
    register_hook();
    register_hook();
}

#[test]
fn test_db_api() {
    let pool = db();
    // 没有初始化时返回 None
    let _ = pool;
}

#[test]
fn test_get_pool_names_api() {
    let names = get_pool_names();
    let _ = names;
}
