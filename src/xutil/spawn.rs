//! 安全异步任务提交工具
//!
//! 提供 fire-and-forget 风格的异步任务提交，
//! 内部自动捕获 panic 并记录日志，避免任务 panic 导致程序崩溃。

use std::any::Any;
use std::future::Future;

/// 安全提交异步任务到 tokio 运行时
///
/// 类似 `tokio::spawn`，但内部自动捕获 panic 并用 `tracing::error!` 记录，
/// 不会因单个任务 panic 导致整个程序崩溃。
///
/// # 参数
/// - `f`: 一个 `Future<Output = ()>`，即无返回值的异步任务
///
/// # 返回值
/// `JoinHandle<()>`，调用方可选择 `.await` 等待完成，也可忽略（fire-and-forget）。
///
/// # Examples
///
/// ```rust
/// use x_one::xutil::spawn_safe;
///
/// #[tokio::main]
/// async fn main() {
///     // fire-and-forget
///     spawn_safe(async {
///         println!("任务执行中");
///     });
///
///     // 也可以 await 等待完成
///     let handle = spawn_safe(async {
///         println!("另一个任务");
///     });
///     handle.await.unwrap();
/// }
/// ```
pub fn spawn_safe<F>(f: F) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        match tokio::spawn(f).await {
            Ok(()) => {}
            Err(e) if e.is_panic() => {
                let msg = extract_panic_message(e.into_panic());
                tracing::error!("spawn_safe task panicked: {msg}");
            }
            Err(e) => {
                tracing::error!("spawn_safe task failed: {e}");
            }
        }
    })
}

/// 从 panic payload 提取可读消息
///
/// 支持 `&str`、`String` 两种常见 panic 消息类型，其他类型返回 `"unknown panic"`。
pub fn extract_panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}
