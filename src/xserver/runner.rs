//! 服务运行和生命周期管理
//!
//! 包含模块初始化（注册 hook + 调用 before_start）和服务运行/停止逻辑。

use super::server::Server;
use crate::error::XOneError;
use crate::xhook;
use crate::xutil;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// 初始化
// ---------------------------------------------------------------------------

/// 缓存初始化结果（成功为 Ok，失败为 Err(错误消息)）
static INIT_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

/// 确保所有模块已初始化（幂等）
///
/// 注册内置模块 hook 并执行 before_start hooks：
/// 1. xconfig（order=1）：加载配置文件
/// 2. xlog（order=2）：设置日志系统
/// 3. xtrace（order=3）：链路追踪
/// 4. xhttp（order=4）：HTTP 客户端
/// 5. xorm（order=5）：数据库连接池
/// 6. xcache（order=6）：本地缓存
/// 7. 用户注册的自定义 hooks
///
/// 初始化只执行一次，后续调用直接返回缓存的结果。
pub fn ensure_init() -> Result<(), XOneError> {
    INIT_RESULT
        .get_or_init(|| {
            register_builtin_hooks();
            invoke_before_start_hooks_with_log().map_err(|e| e.to_string())
        })
        .as_ref()
        .map(|_| ())
        .map_err(|msg| XOneError::Hook(msg.clone()))
}

/// 注册所有内置模块的 hook
fn register_builtin_hooks() {
    crate::xconfig::register_hook();
    crate::xlog::register_hook();
    crate::xtrace::register_hook();
    crate::xhttp::register_hook();
    crate::xorm::register_hook();
    crate::xcache::register_hook();
}

// ---------------------------------------------------------------------------
// 生命周期 hooks（start / stop 对称）
// ---------------------------------------------------------------------------

/// 执行 BeforeStart hooks，记录日志并传播错误
fn invoke_before_start_hooks_with_log() -> Result<(), XOneError> {
    xutil::info_if_enable_debug("********** XOne invoke before start hooks begin **********");
    match xhook::invoke_before_start_hooks() {
        Ok(()) => {
            xutil::info_if_enable_debug(
                "********** XOne invoke before start hooks success **********",
            );
            Ok(())
        }
        Err(e) => {
            xutil::error_if_enable_debug(&format!(
                "********** XOne invoke before start hooks failed, err=[{e}] **********"
            ));
            Err(e)
        }
    }
}

/// 执行 BeforeStop hooks，记录日志并传播错误
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

// ---------------------------------------------------------------------------
// 信号处理
// ---------------------------------------------------------------------------

/// 等待 SIGTERM 信号（仅 unix）
#[cfg(unix)]
async fn wait_for_terminate_signal() {
    if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        sig.recv().await;
    } else {
        // SIGTERM 注册失败时永远等待，退回 SIGINT 兜底
        std::future::pending::<()>().await;
    }
}

/// 非 unix 平台不支持 SIGTERM，永远等待
#[cfg(not(unix))]
async fn wait_for_terminate_signal() {
    std::future::pending::<()>().await;
}

/// 停止服务并记录日志
async fn shutdown_server<S: Server>(server: &S, signal: &str) -> Result<(), XOneError> {
    xutil::info_if_enable_debug(&format!(
        "********** XOne Stop server begin ({signal}) **********"
    ));
    server.stop().await?;
    xutil::info_if_enable_debug("********** XOne Stop server success **********");
    Ok(())
}

/// 合并 server 结果和 stop hook 结果
fn merge_results(
    server_result: Result<(), XOneError>,
    hook_result: Result<(), XOneError>,
) -> Result<(), XOneError> {
    match (server_result, hook_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(se), Err(he)) => Err(XOneError::Multi(vec![se, he])),
    }
}

// ---------------------------------------------------------------------------
// 服务运行
// ---------------------------------------------------------------------------

/// 以异步方式运行服务，阻塞等待退出信号
///
/// 监听 SIGINT (Ctrl+C) 和 SIGTERM（仅 unix），收到信号后优雅停机。
pub async fn run_with_server<S: Server>(server: &S) -> Result<(), XOneError> {
    let server_result = tokio::select! {
        result = server.run() => {
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
            shutdown_server(server, "SIGINT").await
        }
        _ = wait_for_terminate_signal() => {
            shutdown_server(server, "SIGTERM").await
        }
    };

    let stop_hook_result = invoke_before_stop_hooks_safe();
    merge_results(server_result, stop_hook_result)
}

/// 以用户自定义 Server 实现运行
///
/// 初始化所有模块后，以异步方式运行传入的 Server 实现，
/// 阻塞等待退出信号（SIGINT/SIGTERM）。
/// 适用于 consumer/job 等用户自定义的服务。
pub async fn run_server<S: Server>(server: &S) -> Result<(), XOneError> {
    ensure_init()?;
    run_with_server(server).await
}

/// 以 BlockingServer 运行
///
/// 初始化所有模块后，创建 BlockingServer 阻塞等待退出信号。
/// 适用于 consumer/job 等无需监听端口的后台服务。
pub async fn run_blocking_server() -> Result<(), XOneError> {
    let server = super::blocking::BlockingServer::new();
    run_server(&server).await
}
