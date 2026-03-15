use std::time::Duration;
use x_one::xutil::future::XFuture;

#[tokio::test]
async fn test_future_get() {
    let future = XFuture::spawn(|| async { 42 });
    let result = future.get().await;
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_future_get_with_timeout_success() {
    let future = XFuture::spawn(|| async { "hello" });
    let result = future.get_with_timeout(Duration::from_secs(5)).await;
    assert_eq!(result.unwrap(), "hello");
}

#[tokio::test]
async fn test_future_get_with_timeout_expired() {
    let future = XFuture::spawn(|| async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        "never"
    });
    let result = future.get_with_timeout(Duration::from_millis(50)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("timeout"));
}
