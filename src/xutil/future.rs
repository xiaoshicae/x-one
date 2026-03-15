//! 异步 Future 工具
//!
//! 提供类似 Java CompletableFuture 的异步结果抽象。

use std::time::Duration;
use tokio::sync::oneshot;

/// 异步计算结果
///
/// 支持阻塞等待、超时等待和非阻塞检查。
///
/// # Examples
///
/// ```
/// use x_one::xutil::future::XFuture;
///
/// # tokio_test::block_on(async {
/// let future = XFuture::spawn(|| async { 42 });
/// let result = future.get().await;
/// assert_eq!(result.unwrap(), 42);
/// # });
/// ```
pub struct XFuture<T> {
    rx: oneshot::Receiver<T>,
}

impl<T: Send + 'static> XFuture<T> {
    /// 启动异步任务，返回 Future 用于获取结果
    pub fn spawn<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let result = f().await;
            let _ = tx.send(result);
        });
        Self { rx }
    }

    /// 等待异步任务完成
    pub async fn get(self) -> Result<T, String> {
        self.rx
            .await
            .map_err(|_| "async task was cancelled".to_string())
    }

    /// 等待异步任务完成，超时返回 Err
    pub async fn get_with_timeout(self, timeout: Duration) -> Result<T, String> {
        match tokio::time::timeout(timeout, self.rx).await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(_)) => Err("async task was cancelled".to_string()),
            Err(_) => Err("timeout waiting for async result".to_string()),
        }
    }
}
