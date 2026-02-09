use std::time::Duration;
use x_one::xhook::options::*;

#[test]
fn test_default_options() {
    let opts = HookOptions::default();
    assert_eq!(opts.order, 100);
    assert!(opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}

#[test]
fn test_new_equals_default() {
    let opts = HookOptions::new();
    assert_eq!(opts.order, 100);
    assert!(opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}

#[test]
fn test_builder_order() {
    let opts = HookOptions::new().order(10);
    assert_eq!(opts.order, 10);
    assert!(opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}

#[test]
fn test_builder_must_success() {
    let opts = HookOptions::new().order(2).must_success(false);
    assert_eq!(opts.order, 2);
    assert!(!opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}

#[test]
fn test_builder_timeout() {
    let opts = HookOptions::new().timeout(Duration::from_secs(30));
    assert_eq!(opts.order, 100);
    assert_eq!(opts.timeout, Duration::from_secs(30));
}

#[test]
fn test_builder_chain_all() {
    let opts = HookOptions::new()
        .order(5)
        .must_success(false)
        .timeout(Duration::from_secs(10));
    assert_eq!(opts.order, 5);
    assert!(!opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(10));
}
