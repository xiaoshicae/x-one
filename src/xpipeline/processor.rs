//! Pipeline 处理器 trait

use super::Frame;
use crate::XOneError;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// Pipeline 中的处理器接口
///
/// 每个 Processor 在独立 tokio task 中运行，
/// 从 input channel 读取 Frame，处理后写入 output channel。
pub trait Processor: Send + Sync + 'static {
    /// 处理器名称，用于日志和监控
    fn name(&self) -> &str;

    /// 处理逻辑
    ///
    /// 从 input 读取 Frame，处理后写入 output。
    fn process<'a>(
        &'a self,
        input: &'a mut mpsc::Receiver<Box<dyn Frame>>,
        output: &'a mpsc::Sender<Box<dyn Frame>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), XOneError>> + Send + 'a>>;
}
