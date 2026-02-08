use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use x_one::xutil::retry::*;

#[test]
fn test_retry_success_first_try() {
    let result = retry(|| Ok::<(), String>(()), 3, Duration::from_millis(1));
    assert!(result.is_ok());
}

#[test]
fn test_retry_success_after_retries() {
    let counter = AtomicUsize::new(0);
    let result = retry(
        || {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err("not yet".to_string())
            } else {
                Ok(())
            }
        },
        5,
        Duration::from_millis(1),
    );
    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_all_failures() {
    let counter = AtomicUsize::new(0);
    let result = retry(
        || {
            counter.fetch_add(1, Ordering::SeqCst);
            Err("always fail".to_string())
        },
        3,
        Duration::from_millis(1),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "always fail");
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_zero_attempts() {
    let counter = AtomicUsize::new(0);
    let result = retry(
        || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok::<(), String>(())
        },
        0,
        Duration::from_millis(1),
    );
    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_async_success() {
    let result = retry_async(
        || async { Ok::<(), String>(()) },
        3,
        Duration::from_millis(1),
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_async_failure_then_success() {
    let counter = AtomicUsize::new(0);
    let result = retry_async(
        || {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err("not yet".to_string())
                } else {
                    Ok(())
                }
            }
        },
        5,
        Duration::from_millis(1),
    )
    .await;
    assert!(result.is_ok());
}
