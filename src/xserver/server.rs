//! Server trait 定义
//!
//! 实现该 trait 即可接入框架的生命周期管理（`init → run → signal → shutdown`）。
#![allow(async_fn_in_trait)]

use crate::error::XOneError;

/// 服务器 trait
///
/// 实现此 trait 后通过 `run_server(&server)` 运行，
/// 框架自动处理 `init()`、信号监听和 `shutdown()`。
///
/// ```ignore
/// use x_one::{Server, XOneError};
///
/// struct MyServer;
///
/// impl Server for MyServer {
///     async fn run(&self) -> Result<(), XOneError> {
///         // 启动监听或阻塞运行
///         Ok(())
///     }
///     async fn stop(&self) -> Result<(), XOneError> {
///         // 收到退出信号后的清理逻辑
///         Ok(())
///     }
/// }
/// ```
pub trait Server: Send + Sync {
    /// 启动服务（阻塞运行）
    ///
    /// 框架以异步方式调用此方法，同时监听 SIGINT/SIGTERM 信号。
    /// 收到信号后框架会调用 `stop()` 通知服务退出。
    async fn run(&self) -> Result<(), XOneError>;

    /// 停止服务
    ///
    /// 收到退出信号时由框架调用，应在此方法中停止接收新请求、
    /// 通知 `run()` 退出。框架会在 `stop()` 后等待 `run()` 返回。
    async fn stop(&self) -> Result<(), XOneError>;
}
