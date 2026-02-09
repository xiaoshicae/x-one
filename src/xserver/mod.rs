//! Server trait 定义和服务运行逻辑
#![allow(async_fn_in_trait)]

pub mod blocking;

use crate::error::XOneError;
use crate::xhook;
use crate::xutil;

/// 服务器 trait
///
/// 所有服务器实现（AuxmServer, BlockingServer 等）都需要实现此 trait。
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

/// 以异步方式运行服务，阻塞等待退出信号
#[cfg(unix)]
pub async fn run_with_server<S: Server>(server: &S) -> Result<(), XOneError> {
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| XOneError::Server(format!("failed to register SIGTERM: {e}")))?;

    let run_fut = server.run();

    let server_result = tokio::select! {
        result = run_fut => {
            match result {
                Ok(()) => {
                    xutil::warn_if_enable_debug("XOne Run server unexpected stopped");
                    Ok(())
                }
                Err(e) => {
                    Err(XOneError::Server(format!("XOne Run server failed, err=[{e}]")))
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            xutil::info_if_enable_debug("********** XOne Stop server begin (SIGINT) **********");
            safe_invoke_server_stop(server).await?;
            xutil::info_if_enable_debug("********** XOne Stop server success **********");
            Ok(())
        }
        _ = sigterm.recv() => {
            xutil::info_if_enable_debug("********** XOne Stop server begin (SIGTERM) **********");
            safe_invoke_server_stop(server).await?;
            xutil::info_if_enable_debug("********** XOne Stop server success **********");
            Ok(())
        }
    };

    let stop_hook_result = invoke_before_stop_hooks_safe();

    match (server_result, stop_hook_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(e)) => Err(e),
        (Err(server_err), Err(hook_err)) => Err(XOneError::Multi(vec![server_err, hook_err])),
    }
}

/// 以异步方式运行服务，阻塞等待退出信号（Windows 仅支持 Ctrl+C）
#[cfg(not(unix))]
pub async fn run_with_server<S: Server>(server: &S) -> Result<(), XOneError> {
    let run_fut = server.run();

    let server_result = tokio::select! {
        result = run_fut => {
            match result {
                Ok(()) => {
                    xutil::warn_if_enable_debug("XOne Run server unexpected stopped");
                    Ok(())
                }
                Err(e) => {
                    Err(XOneError::Server(format!("XOne Run server failed, err=[{e}]")))
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            xutil::info_if_enable_debug("********** XOne Stop server begin (SIGINT) **********");
            safe_invoke_server_stop(server).await?;
            xutil::info_if_enable_debug("********** XOne Stop server success **********");
            Ok(())
        }
    };

    // 调用 BeforeStop hooks（无论服务结果如何都执行）
    let stop_hook_result = invoke_before_stop_hooks_safe();

    // 合并错误：优先返回服务运行错误，stop hook 错误附加报告
    match (server_result, stop_hook_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(e)) => Err(e),
        (Err(server_err), Err(hook_err)) => Err(XOneError::Multi(vec![server_err, hook_err])),
    }
}

/// 安全调用服务的 stop 方法
pub async fn safe_invoke_server_stop<S: Server>(server: &S) -> Result<(), XOneError> {
    server.stop().await
}

/// 安全调用 BeforeStop hooks，捕获错误
pub fn invoke_before_stop_hooks_safe() -> Result<(), XOneError> {
    xutil::info_if_enable_debug("********** XOne invoke before stop hooks begin **********");
    match xhook::invoke_before_stop_hooks() {
        Ok(()) => {
            xutil::info_if_enable_debug(
                "********** XOne invoke before stop hooks success **********",
            );
            Ok(())
        }
        Err(e) => {
            xutil::error_if_enable_debug(&format!(
                "********** XOne invoke before stop hooks failed, err=[{e}] **********"
            ));
            Err(e)
        }
    }
}
