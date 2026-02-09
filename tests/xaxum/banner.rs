use x_one::xaxum::banner;

#[test]
fn test_version_not_empty() {
    assert!(
        !banner::VERSION.is_empty(),
        "VERSION 应从 Cargo.toml 自动获取，不为空"
    );
}

#[test]
fn test_print_banner_executes() {
    // 验证 print_banner 不会 panic
    banner::print_banner();
}
