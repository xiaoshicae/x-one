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
pub type HookFunc = Box<dyn FnOnce() -> Result<(), XOneError> + Send + 'static>;

/// 内部 Hook 结构体
pub struct Hook {
    pub func: HookFunc,
    pub options: HookOptions,
    pub name: String,
}

/// 全局 Hook 管理器
pub struct HookManager {
    pub before_start_hooks: Vec<Hook>,
    pub before_start_sorted: bool,
    pub before_stop_hooks: Vec<Hook>,
    pub before_stop_sorted: bool,
    pub stop_timeout: Duration,
}

impl Default for HookManager {
    fn default() -> Self {
        Self {
            before_start_hooks: Vec::new(),
            before_start_sorted: true,
            before_stop_hooks: Vec::new(),
            before_stop_sorted: true,
            stop_timeout: Duration::from_secs(DEFAULT_STOP_TIMEOUT_SECS),
        }
    }
}

pub fn manager() -> &'static Mutex<HookManager> {
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

/// 注册 BeforeStart Hook
///
/// # Panics
///
/// 当 Hook 数量超过上限时 panic
pub fn before_start<F>(name: &str, f: F, opts: HookOptions)
where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let mut mgr = manager().lock();
    register_hook(&mut mgr, true, name, f, opts);
}

/// 注册 BeforeStop Hook
///
/// # Panics
///
/// 当 Hook 数量超过上限时 panic
pub fn before_stop<F>(name: &str, f: F, opts: HookOptions)
where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let mut mgr = manager().lock();
    register_hook(&mut mgr, false, name, f, opts);
}

// ... invoke functions ...

fn register_hook<F>(
    mgr: &mut HookManager,
    is_start: bool,
    name: &str,
    f: F,
    opts: HookOptions,
) where
    F: FnOnce() -> Result<(), XOneError> + Send + 'static,
{
    let (hooks, sorted_flag, label) = if is_start {
        (&mut mgr.before_start_hooks, &mut mgr.before_start_sorted, "BeforeStart")
    } else {
        (&mut mgr.before_stop_hooks, &mut mgr.before_stop_sorted, "BeforeStop")
    };

    if hooks.len() >= MAX_HOOK_NUM {
        panic!("XOne {label} hook can not be more than {MAX_HOOK_NUM}");
    }

    hooks.push(Hook {
        func: Box::new(f),
        options: opts,
        name: name.to_string(),
    });
    *sorted_flag = false;
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
                xutil::error_if_enable_debug(&format!(
                    "XOne invoke before stop hook failed, func=[{}], err=[{e}]",
                    h.name,
                ));
                err_msgs.push(format!("func=[{}], err=[{e}]", h.name));
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
        Ok(result) => match result {
            Ok(()) => Ok(()),
            Err(e) => Err(XOneError::Hook(format!(
                "invoke before stop hook failed, {e}"
            ))),
        },
        Err(_) => Err(XOneError::Hook(format!(
            "invoke before stop hook failed, due to timeout after {timeout:?}"
        ))),
    }
}

/// 从管理器中取出排序后的 hooks
fn take_sorted_hooks(is_start: bool) -> Vec<Hook> {
    let mut mgr = manager().lock();
    let mgr = &mut *mgr;

    let (hooks, sorted) = if is_start {
        (&mut mgr.before_start_hooks, &mut mgr.before_start_sorted)
    } else {
        (&mut mgr.before_stop_hooks, &mut mgr.before_stop_sorted)
    };

    if !*sorted {
        hooks.sort_by_key(|h| h.options.order);
        *sorted = true;
    }
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
pub fn reset_hooks() {
    let mut mgr = manager().lock();
    mgr.before_start_hooks.clear();
    mgr.before_start_sorted = true;
    mgr.before_stop_hooks.clear();
    mgr.before_stop_sorted = true;
    mgr.stop_timeout = Duration::from_secs(DEFAULT_STOP_TIMEOUT_SECS);
}
