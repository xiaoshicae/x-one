//! 阻塞式服务器
//!
//! 用于 consumer/job 等服务，以阻塞方式启动，等待退出信号。

use super::Server;
use crate::error::XOneError;
use tokio::sync::watch;

/// 阻塞式服务器
///
/// 通过 watch channel 实现简单的阻塞等待，
/// 调用 `stop()` 时触发 `run()` 返回。
pub struct BlockingServer {
    tx: watch::Sender<bool>,
    rx: watch::Receiver<bool>,
}

impl BlockingServer {
    /// 创建新的阻塞式服务器
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self { tx, rx }
    }

    /// 获取 receiver channel（仅测试用）
    #[doc(hidden)]
    pub fn rx(&self) -> watch::Receiver<bool> {
        self.rx.clone()
    }
}

impl Default for BlockingServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Server for BlockingServer {
    async fn run(&self) -> Result<(), XOneError> {
        let mut rx = self.rx.clone();
        // 等待 stop 信号
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), XOneError> {
        let _ = self.tx.send(true);
        Ok(())
    }
}
