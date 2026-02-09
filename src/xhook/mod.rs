//! xhook - Hook 生命周期管理模块
//!
//! 提供 `before_start` 和 `before_stop` 两类 Hook，
//! 支持排序执行、超时控制和 panic 恢复。

pub mod manager;
pub mod options;

pub use manager::{
    _before_start, _before_stop, invoke_before_start_hooks, invoke_before_stop_hooks, reset_hooks,
};
pub use options::HookOptions;

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
        $crate::xhook::_before_start(concat!(file!(), ":", line!()), $f, $opts)
    };
}

/// 注册 BeforeStop Hook
///
/// 自动以调用位置（file:line）作为 Hook 名称，`HookOptions` 可选(会直接忽略must_invoke_success=true，即失败也不影响后续其它hook的stop)。
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
        $crate::xhook::_before_stop(concat!(file!(), ":", line!()), $f, $opts)
    };
}
