//! 流程监控
//!
//! `Monitor` trait 定义流程执行过程中的回调钩子，`DefaultMonitor` 使用 tracing 输出日志。

use std::time::Duration;

use crate::XOneError;

use super::processor::Dependency;
use super::result::ExecuteResult;

/// 流程监控 trait
///
/// 在流程执行的关键节点接收回调，用于日志记录、指标采集等。
/// 所有方法都有默认空实现，可按需重写。
pub trait Monitor: Send + Sync {
    /// 步骤处理完成时回调
    ///
    /// - `flow_name`: 流程名称
    /// - `processor_name`: 处理器名称
    /// - `dependency`: 依赖类型
    /// - `duration`: 步骤执行耗时
    /// - `err`: 处理失败时的错误
    fn on_process_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        dependency: Dependency,
        duration: Duration,
        err: Option<&XOneError>,
    ) {
        let _ = (flow_name, processor_name, dependency, duration, err);
    }

    /// 步骤回滚完成时回调
    ///
    /// - `flow_name`: 流程名称
    /// - `processor_name`: 处理器名称
    /// - `duration`: 回滚执行耗时
    /// - `err`: 回滚失败时的错误
    fn on_rollback_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        duration: Duration,
        err: Option<&XOneError>,
    ) {
        let _ = (flow_name, processor_name, duration, err);
    }

    /// 流程执行完成时回调
    ///
    /// - `flow_name`: 流程名称
    /// - `duration`: 流程总耗时
    /// - `result`: 执行结果
    fn on_flow_done(&self, flow_name: &str, duration: Duration, result: &ExecuteResult) {
        let _ = (flow_name, duration, result);
    }
}

/// 默认监控器，使用 tracing 输出日志
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use x_one::xflow::{DefaultMonitor, Monitor, Dependency};
///
/// let monitor = DefaultMonitor;
///
/// // 不会 panic，输出 tracing 日志
/// monitor.on_process_done("my_flow", "step1", Dependency::Strong, Duration::from_millis(5), None);
/// monitor.on_process_done("my_flow", "step2", Dependency::Weak, Duration::from_millis(3), None);
/// monitor.on_rollback_done("my_flow", "step1", Duration::from_millis(1), None);
/// ```
pub struct DefaultMonitor;

impl Monitor for DefaultMonitor {
    fn on_process_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        dependency: Dependency,
        duration: Duration,
        err: Option<&XOneError>,
    ) {
        match err {
            None => {
                tracing::debug!(
                    flow = flow_name,
                    processor = processor_name,
                    dependency = %dependency,
                    duration_ms = duration.as_millis() as u64,
                    "process done"
                );
            }
            Some(e) => {
                tracing::warn!(
                    flow = flow_name,
                    processor = processor_name,
                    dependency = %dependency,
                    duration_ms = duration.as_millis() as u64,
                    error = %e,
                    "process failed"
                );
            }
        }
    }

    fn on_rollback_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        duration: Duration,
        err: Option<&XOneError>,
    ) {
        match err {
            None => {
                tracing::debug!(
                    flow = flow_name,
                    processor = processor_name,
                    duration_ms = duration.as_millis() as u64,
                    "rollback done"
                );
            }
            Some(e) => {
                tracing::warn!(
                    flow = flow_name,
                    processor = processor_name,
                    duration_ms = duration.as_millis() as u64,
                    error = %e,
                    "rollback failed"
                );
            }
        }
    }

    fn on_flow_done(&self, flow_name: &str, duration: Duration, result: &ExecuteResult) {
        if result.success() {
            tracing::info!(
                flow = flow_name,
                duration_ms = duration.as_millis() as u64,
                "flow completed successfully"
            );
        } else {
            tracing::error!(
                flow = flow_name,
                duration_ms = duration.as_millis() as u64,
                result = %result,
                "flow failed"
            );
        }
    }
}
