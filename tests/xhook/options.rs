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
fn test_with_order() {
    let opts = HookOptions::with_order(10);
    assert_eq!(opts.order, 10);
    assert!(opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}

#[test]
fn test_with_order_and_must() {
    let opts = HookOptions::with_order_and_must(2, false);
    assert_eq!(opts.order, 2);
    assert!(!opts.must_invoke_success);
    assert_eq!(opts.timeout, Duration::from_secs(5));
}
