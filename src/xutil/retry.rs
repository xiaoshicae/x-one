//! 重试工具
//!
//! 基于 `backon` 封装，内部使用指数退避策略。

use std::time::Duration;

use backon::ExponentialBuilder;

/// 重试执行函数（同步）
///
/// 内部使用指数退避策略，`delay` 为初始间隔，每次翻倍。
/// `max_retries` 为最大重试次数（不含首次执行）。
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// let result = x_one::xutil::retry(|| Ok::<_, String>("ok"), 3, Duration::from_millis(10));
/// assert_eq!(result.unwrap(), "ok");
/// ```
pub fn retry<F, T, E>(f: F, max_retries: usize, delay: Duration) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    use backon::BlockingRetryable;

    f.retry(backoff(max_retries, delay))
        .sleep(std::thread::sleep)
        .call()
}

/// 重试执行函数（异步）
///
/// 内部使用指数退避策略，`delay` 为初始间隔，每次翻倍。
/// `max_retries` 为最大重试次数（不含首次执行）。
///
/// # Examples
///
/// ```
/// # tokio_test::block_on(async {
/// use std::time::Duration;
/// let result = x_one::xutil::retry_async(|| async { Ok::<_, String>("ok") }, 3, Duration::from_millis(10)).await;
/// assert_eq!(result.unwrap(), "ok");
/// # });
/// ```
pub async fn retry_async<F, Fut, T, E>(f: F, max_retries: usize, delay: Duration) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    use backon::Retryable;

    f.retry(backoff(max_retries, delay))
        .sleep(tokio::time::sleep)
        .await
}

// ---- 以下为私有实现 ----

/// 构建指数退避策略
fn backoff(max_retries: usize, delay: Duration) -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(delay)
        .with_max_times(max_retries)
}
