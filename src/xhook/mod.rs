//! xhook - Hook 生命周期管理模块
//!
//! 提供 `before_start` 和 `before_stop` 两类 Hook，
//! 支持排序执行、超时控制和 panic 恢复。

pub mod options;

pub use options::HookOptions;

use crate::error::XOneError;
use crate::xutil;
use parking_lot::Mutex;
use std::panic;
use std::sync::OnceLock;
use std::time::Duration;

/// 默认 stop 超时时间（30 秒）
const DEFAULT_STOP_TIMEOUT_SECS: u64 = 30;
/// Hook 数量上限
const MAX_HOOK_NUM: usize = 1000;

/// Hook 函数类型
type HookFunc = Box<dyn FnOnce() -> Result<(), XOneError> + Send + 'static>;

/// 内部 Hook 结构体
struct Hook {
    func: HookFunc,
    options: HookOptions,
    name: String,
}

/// 全局 Hook 管理器
struct HookManager {
    before_start_hooks: Vec<Hook>,
    before_stop_hooks: Vec<Hook>,
    stop_timeout: Duration,
}

impl Default for HookManager {
    fn default() -> Self {
        Self {
            before_start_hooks: Vec::new(),
            before_stop_hooks: Vec::new(),
            stop_timeout: Duration::from_secs(DEFAULT_STOP_TIMEOUT_SECS),
        }
    }
}

fn manager() -> &'static Mutex<HookManager> {
    static INSTANCE: OnceLock<Mutex<HookManager>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(HookManager::default()))
}

/// 设置 BeforeStop hooks 的超时时间
pub fn set_stop_timeout(timeout: Duration) {
    if timeout > Duration::ZERO {
        let mut mgr = manager().lock();
        mgr.stop_timeout = timeout;
    }
}

/// 获取当前 BeforeStop hooks 的超时时间
pub fn get_stop_timeout() -> Duration {
    let mgr = manager().lock();
    mgr.stop_timeout
}

/// 注册 BeforeStart Hook（内部实现，请使用 `before_start!` 宏）
#[doc(hidden)]
pub fn _before_start<F>(name: &str, f: F, opts: HookOptions)
where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let mut mgr = manager().lock();
    register_hook(&mut mgr, true, name, f, opts);
}

/// 注册 BeforeStop Hook（内部实现，请使用 `before_stop!` 宏）
#[doc(hidden)]
pub fn _before_stop<F>(name: &str, f: F, opts: HookOptions)
where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let mut mgr = manager().lock();
    register_hook(&mut mgr, false, name, f, opts);
}

/// 注册 BeforeStart Hook
///
/// 自动以调用位置（file:line）作为 Hook 名称，`HookOptions` 可选。
///
/// # Examples
///
/// ```ignore
/// // 仅传函数，使用默认选项
/// x_one::before_start!(|| Ok(()));
///
/// // 指定选项
/// x_one::before_start!(|| Ok(()), HookOptions::with_order(10));
/// ```
#[macro_export]
macro_rules! before_start {
    ($f:expr) => {
        $crate::xhook::_before_start(
            concat!(file!(), ":", line!()),
            $f,
            $crate::xhook::HookOptions::default(),
        )
    };
    ($f:expr, $opts:expr) => {
        $crate::xhook::_before_start(
            concat!(file!(), ":", line!()),
            $f,
            $opts,
        )
    };
}

/// 注册 BeforeStop Hook
///
/// 自动以调用位置（file:line）作为 Hook 名称，`HookOptions` 可选。
///
/// # Examples
///
/// ```ignore
/// // 仅传函数，使用默认选项
/// x_one::before_stop!(|| Ok(()));
///
/// // 指定选项
/// x_one::before_stop!(|| Ok(()), HookOptions::with_order(1));
/// ```
#[macro_export]
macro_rules! before_stop {
    ($f:expr) => {
        $crate::xhook::_before_stop(
            concat!(file!(), ":", line!()),
            $f,
            $crate::xhook::HookOptions::default(),
        )
    };
    ($f:expr, $opts:expr) => {
        $crate::xhook::_before_stop(
            concat!(file!(), ":", line!()),
            $f,
            $opts,
        )
    };
}

fn register_hook<F>(
    mgr: &mut HookManager,
    is_start: bool,
    name: &str,
    f: F,
    opts: HookOptions,
) where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let (hooks, label) = if is_start {
        (&mut mgr.before_start_hooks, "BeforeStart")
    } else {
        (&mut mgr.before_stop_hooks, "BeforeStop")
    };

    if hooks.len() >= MAX_HOOK_NUM {
        panic!("XOne {label} hook can not be more than {MAX_HOOK_NUM}");
    }

    hooks.push(Hook {
        func: Box::new(f),
        options: opts,
        name: name.to_string(),
    });
}

/// 执行所有 BeforeStart Hook
pub fn invoke_before_start_hooks() -> Result<(), XOneError> {
    let hooks = take_sorted_hooks(true);

    for h in hooks {
        if let Err(e) = safe_invoke_hook(h.func) {
            if h.options.must_invoke_success {
                xutil::error_if_enable_debug(&format!(
                    "XOne invoke before start hook failed, func=[{}], err=[{e}]",
                    h.name,
                ));
                return Err(XOneError::Hook(format!(
                    "invoke before start hook failed, func=[{}], err=[{e}]",
                    h.name,
                )));
            }
            xutil::warn_if_enable_debug(&format!(
                "XOne invoke before start hook failed, case MustInvokeSuccess=false, func=[{}], err=[{e}]",
                h.name,
            ));
            continue;
        }

        xutil::info_if_enable_debug(&format!(
            "XOne invoke before start hook success, func=[{}]",
            h.name
        ));
    }
    Ok(())
}

/// 执行所有 BeforeStop Hook（带超时控制）
pub fn invoke_before_stop_hooks() -> Result<(), XOneError> {
    let hooks = take_sorted_hooks(false);

    if hooks.is_empty() {
        return Ok(());
    }

    let timeout = {
        let mgr = manager().lock();
        mgr.stop_timeout
    };

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut err_msgs: Vec<String> = Vec::new();

        for h in hooks {
            if let Err(e) = safe_invoke_hook(h.func) {
                if h.options.must_invoke_success {
                    xutil::error_if_enable_debug(&format!(
                        "XOne invoke before stop hook failed, func=[{}], err=[{e}]",
                        h.name,
                    ));
                    err_msgs.push(format!("func=[{}], err=[{e}]", h.name));
                } else {
                    xutil::warn_if_enable_debug(&format!(
                        "XOne invoke before stop hook failed, case MustInvokeSuccess=false, func=[{}], err=[{e}]",
                        h.name,
                    ));
                }
                continue;
            }

            xutil::info_if_enable_debug(&format!(
                "XOne invoke before stop hook success, func=[{}]",
                h.name
            ));
        }

        let result = if err_msgs.is_empty() {
            Ok(())
        } else {
            Err(err_msgs.join("; "))
        };
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(XOneError::Hook(format!(
            "invoke before stop hook failed, {e}"
        ))),
        Err(_) => Err(XOneError::Hook(format!(
            "invoke before stop hook failed, due to timeout after {timeout:?}"
        ))),
    }
}

/// 从管理器中取出排序后的 hooks
fn take_sorted_hooks(is_start: bool) -> Vec<Hook> {
    let mut mgr = manager().lock();
    let hooks = if is_start {
        &mut mgr.before_start_hooks
    } else {
        &mut mgr.before_stop_hooks
    };
    hooks.sort_by_key(|h| h.options.order);
    std::mem::take(hooks)
}

/// 安全执行 Hook 函数，捕获 panic
fn safe_invoke_hook(f: HookFunc) -> Result<(), XOneError> {
    panic::catch_unwind(panic::AssertUnwindSafe(f)).unwrap_or_else(|e| {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        Err(XOneError::Hook(format!("panic occurred, {msg}")))
    })
}

/// 重置 Hook 管理器（仅测试用）
#[doc(hidden)]
pub fn reset_hooks() {
    let mut mgr = manager().lock();
    mgr.before_start_hooks.clear();
    mgr.before_stop_hooks.clear();
    mgr.stop_timeout = Duration::from_secs(DEFAULT_STOP_TIMEOUT_SECS);
}
