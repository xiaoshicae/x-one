//! 流程编排器
//!
//! `Flow<T>` 顺序执行处理器，支持强/弱依赖模型和自动逆序回滚。

use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration, Instant};

use crate::XOneError;
use crate::xutil::extract_panic_message;

use super::monitor::{DefaultMonitor, Monitor};
use super::processor::{Dependency, Processor};
use super::result::{ExecuteResult, StepError};

/// 流程编排器
///
/// 顺序执行一组处理器，支持：
/// - **强依赖**：失败时中断流程并逆序回滚已成功的步骤
/// - **弱依赖**：失败时记录错误并继续执行后续步骤
/// - **panic 安全**：自动捕获处理器中的 panic
/// - **监控回调**：在关键节点通知 `Monitor`（需启用 `enable_monitor`）
///
/// # Examples
///
/// ```
/// use x_one::xflow::{Flow, Step};
///
/// let flow = Flow::new("order")
///     .step(Step::new("validate").process(|data: &mut Vec<String>| {
///         data.push("validated".into());
///         Ok(())
///     }))
///     .step(Step::new("save").process(|data: &mut Vec<String>| {
///         data.push("saved".into());
///         Ok(())
///     }));
///
/// let mut data = Vec::new();
/// let result = flow.execute(&mut data);
/// assert!(result.success());
/// assert_eq!(data, vec!["validated", "saved"]);
/// ```
pub struct Flow<T> {
    name: String,
    processors: Vec<Box<dyn Processor<T>>>,
    enable_monitor: bool,
    monitor: Option<Box<dyn Monitor>>,
}

impl<T: Send + Sync> Flow<T> {
    /// 创建新的流程编排器
    ///
    /// 默认不启用监控（零开销），调用 `enable_monitor()` 或 `monitor()` 启用。
    ///
    /// # Examples
    ///
    /// ```
    /// use x_one::xflow::Flow;
    ///
    /// let flow = Flow::<()>::new("my_flow");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            processors: Vec::new(),
            enable_monitor: false,
            monitor: None,
        }
    }

    /// 添加处理器
    pub fn step(mut self, processor: impl Processor<T> + 'static) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    /// 启用监控（使用 `DefaultMonitor`，除非已通过 `monitor()` 设置自定义监控器）
    pub fn enable_monitor(mut self) -> Self {
        self.enable_monitor = true;
        self
    }

    /// 设置自定义监控器（自动启用监控）
    pub fn monitor(mut self, monitor: impl Monitor + 'static) -> Self {
        self.monitor = Some(Box::new(monitor));
        self.enable_monitor = true;
        self
    }

    /// 解析最终监控器：未启用返回 None，启用时返回自定义或默认监控器
    fn resolve_monitor(&self) -> Option<&dyn Monitor> {
        if !self.enable_monitor {
            return None;
        }
        Some(
            self.monitor
                .as_deref()
                .unwrap_or(&DefaultMonitor as &dyn Monitor),
        )
    }

    /// 执行流程
    ///
    /// 顺序执行所有处理器：
    /// - Strong 失败 → 中断 + 逆序回滚已成功步骤
    /// - Weak 失败 → 记录跳过错误 + 继续
    /// - panic → 视为错误处理
    pub fn execute(&self, data: &mut T) -> ExecuteResult {
        let monitor = self.resolve_monitor();
        let flow_start = monitor.map(|_| Instant::now());

        let mut result = ExecuteResult::ok();
        let mut succeeded_indices: Vec<usize> = Vec::new();

        for (i, processor) in self.processors.iter().enumerate() {
            let name = processor.name().to_string();
            let dependency = processor.dependency();

            let step_start = monitor.map(|_| Instant::now());
            let outcome = safe_process(processor.as_ref(), data);
            let step_duration = step_start.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);

            match outcome {
                Ok(()) => {
                    if let Some(m) = monitor {
                        m.on_process_done(&self.name, &name, dependency, step_duration, None);
                    }
                    succeeded_indices.push(i);
                }
                Err(e) => {
                    if let Some(m) = monitor {
                        m.on_process_done(&self.name, &name, dependency, step_duration, Some(&e));
                    }

                    match dependency {
                        Dependency::Strong => {
                            result.err = Some(StepError::new(name, dependency, e));
                            // 逆序回滚已成功步骤
                            self.rollback(data, &succeeded_indices, &mut result, monitor);
                            let flow_duration =
                                flow_start.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);
                            if let Some(m) = monitor {
                                m.on_flow_done(&self.name, flow_duration, &result);
                            }
                            return result;
                        }
                        Dependency::Weak => {
                            result
                                .skipped_errors
                                .push(StepError::new(name, dependency, e));
                            // 弱依赖失败也加入 succeeded，回滚时需要回滚
                            succeeded_indices.push(i);
                        }
                    }
                }
            }
        }

        let flow_duration = flow_start.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);
        if let Some(m) = monitor {
            m.on_flow_done(&self.name, flow_duration, &result);
        }
        result
    }

    /// 逆序回滚已成功步骤
    fn rollback(
        &self,
        data: &mut T,
        succeeded_indices: &[usize],
        result: &mut ExecuteResult,
        monitor: Option<&dyn Monitor>,
    ) {
        result.rolled = true;

        for &idx in succeeded_indices.iter().rev() {
            let processor = &self.processors[idx];
            let name = processor.name().to_string();

            let step_start = monitor.map(|_| Instant::now());
            let outcome = safe_rollback(processor.as_ref(), data);
            let step_duration = step_start.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);

            match outcome {
                Ok(()) => {
                    if let Some(m) = monitor {
                        m.on_rollback_done(&self.name, &name, step_duration, None);
                    }
                }
                Err(e) => {
                    if let Some(m) = monitor {
                        m.on_rollback_done(&self.name, &name, step_duration, Some(&e));
                    }
                    result
                        .rollback_errors
                        .push(StepError::new(name, processor.dependency(), e));
                }
            }
        }
    }
}

/// 安全执行 process，捕获 panic
fn safe_process<T>(processor: &dyn Processor<T>, data: &mut T) -> Result<(), XOneError> {
    panic::catch_unwind(AssertUnwindSafe(|| processor.process(data))).unwrap_or_else(|e| {
        let msg = extract_panic_message(e);
        Err(XOneError::Other(format!(
            "panic in process [{}]: {msg}",
            processor.name()
        )))
    })
}

/// 安全执行 rollback，捕获 panic
fn safe_rollback<T>(processor: &dyn Processor<T>, data: &mut T) -> Result<(), XOneError> {
    panic::catch_unwind(AssertUnwindSafe(|| processor.rollback(data))).unwrap_or_else(|e| {
        let msg = extract_panic_message(e);
        Err(XOneError::Other(format!(
            "panic in rollback [{}]: {msg}",
            processor.name()
        )))
    })
}
