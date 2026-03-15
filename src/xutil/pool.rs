//! 异步任务池
//!
//! 提供固定 worker 数量的异步任务池。

use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// 默认任务池 worker 数量
pub const DEFAULT_POOL_SIZE: usize = 100;

/// 任务类型别名
type BoxTask = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// 固定大小异步任务池
///
/// 通过 `submit` 提交任务，后台 tokio task 并发执行。
///
/// # Examples
///
/// ```
/// use x_one::xutil::pool::Pool;
///
/// # tokio_test::block_on(async {
/// let pool = Pool::new(4);
/// pool.submit(|| async {
///     // 执行异步任务
/// });
/// pool.shutdown().await;
/// # });
/// ```
pub struct Pool {
    tx: mpsc::Sender<BoxTask>,
}

impl Pool {
    /// 创建指定 worker 数量的任务池
    pub fn new(worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        let (tx, rx) = mpsc::channel::<BoxTask>(worker_count * 16);

        let rx = std::sync::Arc::new(tokio::sync::Mutex::new(rx));

        for _ in 0..worker_count {
            let rx = rx.clone();
            tokio::spawn(async move {
                loop {
                    let task = {
                        let mut rx = rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some(f) => f().await,
                        None => break,
                    }
                }
            });
        }

        Self { tx }
    }

    /// 提交异步任务
    pub fn submit<F, Fut>(&self, f: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let boxed: BoxTask = Box::new(move || Box::pin(f()));
        let _ = self.tx.try_send(boxed);
    }

    /// 关闭任务池
    ///
    /// 等待 sender 端所有引用被 drop，worker 自动退出。
    pub async fn shutdown(self) {
        drop(self.tx);
    }
}
