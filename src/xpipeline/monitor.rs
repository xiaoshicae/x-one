//! Pipeline 监控接口

use std::time::Duration;

use super::result::RunResult;
use crate::XOneError;

/// 处理器执行完成事件
pub struct StepEvent<'a> {
    /// Pipeline 名称
    pub pipeline_name: &'a str,
    /// 处理器名称
    pub processor_name: &'a str,
    /// 错误（无错误为 None）
    pub err: Option<&'a XOneError>,
    /// 执行耗时
    pub duration: Duration,
}

/// Pipeline 执行完成事件
pub struct PipelineEvent<'a> {
    /// Pipeline 名称
    pub pipeline_name: &'a str,
    /// 执行结果
    pub result: &'a RunResult,
    /// 总耗时
    pub duration: Duration,
}

/// 监控回调接口
///
/// Pipeline 可注入自定义实现以观测执行过程。
/// 注意：回调从 tokio task 中调用，实现必须线程安全。
pub trait Monitor: Send + Sync {
    /// 处理器执行完成时调用
    fn on_processor_done(&self, event: &StepEvent<'_>);
    /// Pipeline 执行完成时调用
    fn on_pipeline_done(&self, event: &PipelineEvent<'_>);
}

/// 默认监控实现，使用 tracing 打印日志
pub struct DefaultMonitor;

impl Monitor for DefaultMonitor {
    fn on_processor_done(&self, event: &StepEvent<'_>) {
        if let Some(err) = event.err {
            tracing::warn!(
                pipeline = event.pipeline_name,
                processor = event.processor_name,
                duration = ?event.duration,
                error = %err,
                "pipeline processor failed"
            );
        } else {
            tracing::info!(
                pipeline = event.pipeline_name,
                processor = event.processor_name,
                duration = ?event.duration,
                "pipeline processor done"
            );
        }
    }

    fn on_pipeline_done(&self, event: &PipelineEvent<'_>) {
        let status = if event.result.success() {
            "success"
        } else {
            "failed"
        };
        tracing::info!(
            pipeline = event.pipeline_name,
            duration = ?event.duration,
            status,
            "pipeline done"
        );
    }
}
