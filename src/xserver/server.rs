//! Server trait 定义
#![allow(async_fn_in_trait)]

use crate::error::XOneError;

/// 服务器 trait
///
/// 所有服务器实现（AxumServer, BlockingServer 等）都需要实现此 trait。
pub trait Server: Send + Sync {
    /// 启动服务
    ///
    /// 建议以阻塞方式运行，框架会以异步方式运行服务，
    /// 且阻塞等待退出信号。
    async fn run(&self) -> Result<(), XOneError>;

    /// 停止服务
    ///
    /// 建议放一些资源清理逻辑。
    async fn stop(&self) -> Result<(), XOneError>;
}
