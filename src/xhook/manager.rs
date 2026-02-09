//! Hook 管理器
//!
//! 提供 Hook 注册、排序执行、超时控制和 panic 恢复。

use crate::error::XOneError;
use crate::xutil;
use parking_lot::Mutex;
use std::panic;
use std::sync::OnceLock;
use std::time::Duration;

use super::HookOptions;

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
#[derive(Default)]
struct HookManager {
    before_start_hooks: Vec<Hook>,
    before_stop_hooks: Vec<Hook>,
}

fn manager() -> &'static Mutex<HookManager> {
    static INSTANCE: OnceLock<Mutex<HookManager>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(HookManager::default()))
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

fn register_hook<F>(mgr: &mut HookManager, is_start: bool, name: &str, f: F, opts: HookOptions)
where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let (hooks, label) = if is_start {
        (&mut mgr.before_start_hooks, "BeforeStart")
    } else {
        (&mut mgr.before_stop_hooks, "BeforeStop")
    };

    if hooks.len() >= MAX_HOOK_NUM {
        xutil::error_if_enable_debug(&format!(
            "XOne {label} hook limit reached ({MAX_HOOK_NUM}), skip register: {name}"
        ));
        return;
    }

    hooks.push(Hook {
        func: Box::new(f),
        options: opts,
        name: name.to_string(),
    });
}

/// 执行所有 BeforeStart Hook（带逐个超时控制）
pub fn invoke_before_start_hooks() -> Result<(), XOneError> {
    let hooks = take_sorted_hooks(true);

    for h in hooks {
        match invoke_hook_with_timeout(h.func, h.options.timeout) {
            Ok(()) => {
                xutil::info_if_enable_debug(&format!(
                    "XOne invoke before start hook success, func=[{}]",
                    h.name
                ));
            }
            Err(e) => {
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
            }
        }
    }
    Ok(())
}

/// 执行所有 BeforeStop Hook（带逐个超时控制）
pub fn invoke_before_stop_hooks() -> Result<(), XOneError> {
    let hooks = take_sorted_hooks(false);

    if hooks.is_empty() {
        return Ok(());
    }

    let mut err_msgs: Vec<String> = Vec::new();

    for h in hooks {
        match invoke_hook_with_timeout(h.func, h.options.timeout) {
            Ok(()) => {
                xutil::info_if_enable_debug(&format!(
                    "XOne invoke before stop hook success, func=[{}]",
                    h.name
                ));
            }
            Err(e) => {
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
            }
        }
    }

    if err_msgs.is_empty() {
        Ok(())
    } else {
        Err(XOneError::Hook(format!(
            "invoke before stop hook failed, {}",
            err_msgs.join("; ")
        )))
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

/// 带超时执行 Hook 函数，同时捕获 panic
fn invoke_hook_with_timeout(f: HookFunc, timeout: Duration) -> Result<(), XOneError> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = safe_invoke_hook(f);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Err(XOneError::Hook(format!("timeout after {timeout:?}")))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(XOneError::Hook(
            "hook thread panicked or exited unexpectedly".to_string(),
        )),
    }
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
}
