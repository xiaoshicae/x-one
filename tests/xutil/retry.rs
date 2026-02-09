use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use x_one::xutil::retry::*;

#[test]
fn test_retry_success_first_try() {
    let result = retry(|| Ok::<_, String>("ok"), 3, Duration::from_millis(1));
    assert_eq!(result.unwrap(), "ok");
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
                Ok("done")
            }
        },
        4, // 最多重试 4 次（首次 + 4 = 5 次机会）
        Duration::from_millis(1),
    );
    assert_eq!(result.unwrap(), "done");
    assert_eq!(counter.load(Ordering::SeqCst), 3); // 第 3 次成功
}

#[test]
fn test_retry_all_failures() {
    let counter = AtomicUsize::new(0);
    let result = retry(
        || {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>("always fail".to_string())
        },
        2, // 最多重试 2 次（首次 + 2 = 3 次执行）
        Duration::from_millis(1),
    );
    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_zero_retries_executes_once() {
    let counter = AtomicUsize::new(0);
    let result = retry(
        || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok::<_, String>("once")
        },
        0, // 不重试，只执行一次
        Duration::from_millis(1),
    );
    assert_eq!(result.unwrap(), "once");
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_async_success() {
    let result = retry_async(
        || async { Ok::<_, String>("async ok") },
        3,
        Duration::from_millis(1),
    )
    .await;
    assert_eq!(result.unwrap(), "async ok");
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
                    Ok("async done")
                }
            }
        },
        4,
        Duration::from_millis(1),
    )
    .await;
    assert_eq!(result.unwrap(), "async done");
}
