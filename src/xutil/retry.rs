//! 重试工具

use std::time::Duration;

/// 重试执行函数指定次数
///
/// 如果 `attempts` 为 0，则只执行一次（不重试）。
/// 每次失败后等待 `sleep` 时长再重试。
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// let result = x_one::xutil::retry(|| Ok::<(), String>(()), 3, Duration::from_millis(10));
/// assert!(result.is_ok());
/// ```
pub fn retry<F, E>(f: F, attempts: usize, sleep: Duration) -> Result<(), E>
where
    F: Fn() -> Result<(), E>,
{
    let actual_attempts = attempts.max(1);

    let mut last_err = None;
    for i in 0..actual_attempts {
        match f() {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if i + 1 < actual_attempts && !sleep.is_zero() {
                    std::thread::sleep(sleep);
                }
            }
        }
    }

    Err(last_err.expect("at least one attempt"))
}

/// 异步重试执行函数指定次数
///
/// 如果 `attempts` 为 0，则只执行一次（不重试）。
/// 每次失败后等待 `sleep` 时长再重试。
pub async fn retry_async<F, Fut, E>(f: F, attempts: usize, sleep: Duration) -> Result<(), E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), E>>,
{
    let actual_attempts = attempts.max(1);

    let mut last_err = None;
    for i in 0..actual_attempts {
        match f().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if i + 1 < actual_attempts && !sleep.is_zero() {
                    tokio::time::sleep(sleep).await;
                }
            }
        }
    }

    Err(last_err.expect("at least one attempt"))
}
