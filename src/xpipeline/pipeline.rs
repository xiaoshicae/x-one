//! Pipeline 编排器

use std::time::Instant;

use tokio::sync::mpsc;

use super::config::PipelineConfig;
use super::frame::Frame;
use super::monitor::{DefaultMonitor, Monitor, PipelineEvent, StepEvent};
use super::processor::Processor;
use super::result::{RunResult, StepError};
use crate::XOneError;

/// Pipeline 运行返回类型
type PipelineOutput = (
    mpsc::Sender<Box<dyn Frame>>,
    mpsc::Receiver<Box<dyn Frame>>,
    tokio::task::JoinHandle<RunResult>,
);

/// 流式 channel 编排器
///
/// 将多个 Processor 串联成 channel 链，每个 Processor 在独立 tokio task 中运行。
pub struct Pipeline {
    name: String,
    processors: Vec<Box<dyn Processor>>,
    config: PipelineConfig,
    monitor: Option<Box<dyn Monitor>>,
}

impl Pipeline {
    /// 创建新的 Pipeline
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            processors: Vec::new(),
            config: PipelineConfig::default(),
            monitor: None,
        }
    }

    /// 添加处理器
    pub fn processor(mut self, p: impl Processor) -> Self {
        self.processors.push(Box::new(p));
        self
    }

    /// 设置配置
    pub fn config(mut self, config: PipelineConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置自定义监控器
    pub fn monitor(mut self, monitor: impl Monitor + 'static) -> Self {
        self.monitor = Some(Box::new(monitor));
        self
    }

    /// 启用默认监控（DefaultMonitor）
    pub fn enable_monitor(mut self) -> Self {
        self.monitor = Some(Box::new(DefaultMonitor));
        self
    }

    /// 启动 Pipeline
    ///
    /// 返回 (input_sender, output_receiver, join_handle)：
    /// - `input_sender`: 向第一个 Processor 发送 Frame
    /// - `output_receiver`: 从最后一个 Processor 接收 Frame
    /// - `join_handle`: 等待所有 Processor 完成并返回 RunResult
    #[allow(clippy::type_complexity)]
    pub fn run(self) -> PipelineOutput {
        let buffer_size = self.config.buffer_size;

        if self.processors.is_empty() {
            let (_tx, rx) = mpsc::channel(1);
            let handle = tokio::spawn(async { RunResult::default() });
            return (mpsc::channel(1).0, rx, handle);
        }

        let (input_tx, first_rx) = mpsc::channel(buffer_size);
        let (last_tx, output_rx) = mpsc::channel(buffer_size);

        let handle = tokio::spawn(run_pipeline(
            self.name,
            self.processors,
            first_rx,
            last_tx,
            buffer_size,
            self.config.disable_monitor,
            self.monitor,
        ));

        (input_tx, output_rx, handle)
    }
}

/// 内部运行逻辑
async fn run_pipeline(
    name: String,
    processors: Vec<Box<dyn Processor>>,
    first_rx: mpsc::Receiver<Box<dyn Frame>>,
    last_tx: mpsc::Sender<Box<dyn Frame>>,
    buffer_size: usize,
    disable_monitor: bool,
    monitor: Option<Box<dyn Monitor>>,
) -> RunResult {
    let pipeline_start = Instant::now();
    let proc_count = processors.len();

    // 创建中间 channel 链
    let mut receivers = Vec::with_capacity(proc_count);
    let mut senders = Vec::with_capacity(proc_count);

    receivers.push(first_rx);

    for _ in 1..proc_count {
        let (tx, rx) = mpsc::channel(buffer_size);
        senders.push(tx);
        receivers.push(rx);
    }
    senders.push(last_tx);

    // 启动所有 Processor task
    let mut handles = Vec::with_capacity(proc_count);

    for (processor, (mut rx, tx)) in processors
        .into_iter()
        .zip(receivers.into_iter().zip(senders.into_iter()))
    {
        let handle = tokio::spawn(async move {
            let proc_name = processor.name().to_string();
            let start = Instant::now();
            let result = processor.process(&mut rx, &tx).await;
            let duration = start.elapsed();
            drop(tx);
            (proc_name, result, duration)
        });
        handles.push(handle);
    }

    // 等待所有 Processor 完成
    let mut result = RunResult::default();
    let active_monitor = if disable_monitor {
        None
    } else {
        monitor.as_deref()
    };

    for handle in handles {
        match handle.await {
            Ok((proc_name, proc_result, duration)) => {
                let err = proc_result.err();

                if let Some(m) = active_monitor {
                    m.on_processor_done(&StepEvent {
                        pipeline_name: &name,
                        processor_name: &proc_name,
                        err: err.as_ref(),
                        duration,
                    });
                }

                if let Some(e) = err {
                    result.errors.push(StepError {
                        processor_name: proc_name,
                        err: e,
                    });
                }
            }
            Err(join_err) => {
                // task panic 或被取消
                let proc_name = "unknown".to_string();
                let err = if join_err.is_panic() {
                    XOneError::Other("processor task panicked".to_string())
                } else {
                    XOneError::Other("processor task cancelled".to_string())
                };

                if let Some(m) = active_monitor {
                    m.on_processor_done(&StepEvent {
                        pipeline_name: &name,
                        processor_name: &proc_name,
                        err: Some(&err),
                        duration: pipeline_start.elapsed(),
                    });
                }

                result.errors.push(StepError {
                    processor_name: proc_name,
                    err,
                });
            }
        }
    }

    if let Some(m) = active_monitor {
        m.on_pipeline_done(&PipelineEvent {
            pipeline_name: &name,
            result: &result,
            duration: pipeline_start.elapsed(),
        });
    }

    result
}
