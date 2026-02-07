use parking_lot::Mutex;
use serial_test::serial;
use std::time::Duration;
use x_one::xhook::*;
use x_one::XOneError;

#[test]
#[serial]
fn test_before_start_hook_success() {
    reset_hooks();
    before_start("test_hook", || Ok(()), HookOptions::default());
    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_before_start_hook_order() {
    reset_hooks();
    let order_log = std::sync::Arc::new(Mutex::new(Vec::new()));

    let log1 = order_log.clone();
    before_start(
        "hook_3",
        move || {
            log1.lock().push(3);
            Ok(())
        },
        HookOptions::with_order(3),
    );

    let log2 = order_log.clone();
    before_start(
        "hook_1",
        move || {
            log2.lock().push(1);
            Ok(())
        },
        HookOptions::with_order(1),
    );

    let log3 = order_log.clone();
    before_start(
        "hook_2",
        move || {
            log3.lock().push(2);
            Ok(())
        },
        HookOptions::with_order(2),
    );

    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
    assert_eq!(*order_log.lock(), vec![1, 2, 3]);
}

#[test]
#[serial]
fn test_before_start_hook_error_must_success() {
    reset_hooks();
    before_start(
        "failing_hook",
        || Err(XOneError::Other("test error".to_string())),
        HookOptions::default(),
    );
    let result = invoke_before_start_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("test error"));
}

#[test]
#[serial]
fn test_before_start_hook_error_not_must_success() {
    reset_hooks();
    let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    before_start(
        "failing_hook",
        || Err(XOneError::Other("test error".to_string())),
        HookOptions::with_order_and_must(1, false),
    );

    let exec = executed.clone();
    before_start(
        "next_hook",
        move || {
            exec.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
        HookOptions::with_order(2),
    );

    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
    assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
#[serial]
fn test_before_start_hook_panic_recovery() {
    reset_hooks();
    before_start("panicking_hook", || panic!("test panic"), HookOptions::default());
    let result = invoke_before_start_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("panic occurred"));
}

#[test]
#[serial]
fn test_before_stop_hook_success() {
    reset_hooks();
    before_stop("stop_hook", || Ok(()), HookOptions::default());
    let result = invoke_before_stop_hooks();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_before_stop_hook_error() {
    reset_hooks();
    before_stop(
        "stop_hook",
        || Err(XOneError::Other("stop error".to_string())),
        HookOptions::default(),
    );
    let result = invoke_before_stop_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("stop error"));
}

#[test]
#[serial]
fn test_before_stop_hook_timeout() {
    reset_hooks();
    set_stop_timeout(Duration::from_millis(100));
    before_stop(
        "slow_hook",
        || {
            std::thread::sleep(Duration::from_secs(5));
            Ok(())
        },
        HookOptions::default(),
    );
    let result = invoke_before_stop_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}

#[test]
#[serial]
fn test_before_stop_hook_panic_recovery() {
    reset_hooks();
    before_stop("panicking_hook", || panic!("stop panic"), HookOptions::default());
    let result = invoke_before_stop_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("panic occurred"));
}

#[test]
#[serial]
fn test_before_stop_empty_hooks() {
    reset_hooks();
    let result = invoke_before_stop_hooks();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_set_stop_timeout() {
    reset_hooks();
    set_stop_timeout(Duration::from_secs(60));
    let mgr = manager().lock();
    assert_eq!(mgr.stop_timeout, Duration::from_secs(60));
}

#[test]
#[serial]
fn test_set_stop_timeout_zero_ignored() {
    reset_hooks();
    let original = {
        let mgr = manager().lock();
        mgr.stop_timeout
    };
    set_stop_timeout(Duration::ZERO);
    let current = {
        let mgr = manager().lock();
        mgr.stop_timeout
    };
    assert_eq!(original, current);
}
