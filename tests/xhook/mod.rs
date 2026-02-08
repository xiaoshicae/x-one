use parking_lot::Mutex;
use serial_test::serial;
use std::time::Duration;
use x_one::xhook::*;
use x_one::XOneError;

#[test]
#[serial]
fn test_before_start_hook_success() {
    reset_hooks();
    x_one::before_start!(|| Ok(()));
    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_before_start_hook_order() {
    reset_hooks();
    let order_log = std::sync::Arc::new(Mutex::new(Vec::new()));

    let log1 = order_log.clone();
    x_one::before_start!(
        move || {
            log1.lock().push(3);
            Ok(())
        },
        HookOptions::with_order(3)
    );

    let log2 = order_log.clone();
    x_one::before_start!(
        move || {
            log2.lock().push(1);
            Ok(())
        },
        HookOptions::with_order(1)
    );

    let log3 = order_log.clone();
    x_one::before_start!(
        move || {
            log3.lock().push(2);
            Ok(())
        },
        HookOptions::with_order(2)
    );

    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
    assert_eq!(*order_log.lock(), vec![1, 2, 3]);
}

#[test]
#[serial]
fn test_before_start_hook_error_must_success() {
    reset_hooks();
    x_one::before_start!(|| Err(XOneError::Other("test error".to_string())));
    let result = invoke_before_start_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("test error"));
}

#[test]
#[serial]
fn test_before_start_hook_error_not_must_success() {
    reset_hooks();
    let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    x_one::before_start!(
        || Err(XOneError::Other("test error".to_string())),
        HookOptions::with_order_and_must(1, false)
    );

    let exec = executed.clone();
    x_one::before_start!(
        move || {
            exec.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
        HookOptions::with_order(2)
    );

    let result = invoke_before_start_hooks();
    assert!(result.is_ok());
    assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
#[serial]
fn test_before_start_hook_panic_recovery() {
    reset_hooks();
    x_one::before_start!(|| panic!("test panic"));
    let result = invoke_before_start_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("panic occurred"));
}

#[test]
#[serial]
fn test_before_stop_hook_success() {
    reset_hooks();
    x_one::before_stop!(|| Ok(()));
    let result = invoke_before_stop_hooks();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_before_stop_hook_error() {
    reset_hooks();
    x_one::before_stop!(|| Err(XOneError::Other("stop error".to_string())));
    let result = invoke_before_stop_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("stop error"));
}

#[test]
#[serial]
fn test_before_stop_hook_timeout() {
    reset_hooks();
    x_one::before_stop!(
        || {
            std::thread::sleep(Duration::from_secs(5));
            Ok(())
        },
        HookOptions {
            timeout: Duration::from_millis(100),
            ..HookOptions::default()
        }
    );
    let result = invoke_before_stop_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}

#[test]
#[serial]
fn test_before_stop_hook_panic_recovery() {
    reset_hooks();
    x_one::before_stop!(|| panic!("stop panic"));
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
fn test_before_stop_hook_error_not_must_success() {
    reset_hooks();
    let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    x_one::before_stop!(
        || Err(XOneError::Other("stop error".to_string())),
        HookOptions::with_order_and_must(1, false)
    );

    let exec = executed.clone();
    x_one::before_stop!(
        move || {
            exec.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
        HookOptions::with_order(2)
    );

    let result = invoke_before_stop_hooks();
    assert!(result.is_ok());
    assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
#[serial]
fn test_before_start_hook_timeout() {
    reset_hooks();
    x_one::before_start!(
        || {
            std::thread::sleep(Duration::from_secs(5));
            Ok(())
        },
        HookOptions {
            timeout: Duration::from_millis(100),
            ..HookOptions::default()
        }
    );
    let result = invoke_before_start_hooks();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}
