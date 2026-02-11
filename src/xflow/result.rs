//! 流程执行结果类型
//!
//! 定义 `StepError`（步骤级错误）和 `ExecuteResult`（流程级结果）。

use std::fmt;

use crate::XOneError;

use super::processor::Dependency;

/// 步骤级错误，包含处理器名称、依赖类型和原始错误
///
/// # Examples
///
/// ```
/// use x_one::xflow::{StepError, Dependency};
/// use x_one::XOneError;
///
/// let err = StepError::new("validate", Dependency::Strong, XOneError::Other("bad input".into()));
/// assert_eq!(err.processor_name(), "validate");
/// assert_eq!(err.dependency(), Dependency::Strong);
/// assert!(err.to_string().contains("validate"));
/// ```
#[derive(Debug)]
pub struct StepError {
    processor_name: String,
    dependency: Dependency,
    err: XOneError,
}

impl StepError {
    /// 创建步骤错误
    pub fn new(processor_name: impl Into<String>, dependency: Dependency, err: XOneError) -> Self {
        Self {
            processor_name: processor_name.into(),
            dependency,
            err,
        }
    }

    /// 处理器名称
    pub fn processor_name(&self) -> &str {
        &self.processor_name
    }

    /// 依赖类型
    pub fn dependency(&self) -> Dependency {
        self.dependency
    }

    /// 原始错误引用
    pub fn err(&self) -> &XOneError {
        &self.err
    }
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "step [{}] ({}) failed: {}",
            self.processor_name, self.dependency, self.err
        )
    }
}

impl std::error::Error for StepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.err)
    }
}

/// 流程执行结果
///
/// 包含执行是否成功、主错误、弱依赖跳过的错误、回滚错误等信息。
///
/// # Examples
///
/// ```
/// use x_one::xflow::Flow;
///
/// // 执行空流程得到成功结果
/// let mut data = ();
/// let result = Flow::<()>::new("test").execute(&mut data);
/// assert!(result.success());
/// assert!(result.err().is_none());
/// assert!(!result.has_skipped_errors());
/// assert!(!result.has_rollback_errors());
/// assert!(!result.rolled());
/// ```
pub struct ExecuteResult {
    pub(crate) err: Option<StepError>,
    pub(crate) skipped_errors: Vec<StepError>,
    pub(crate) rollback_errors: Vec<StepError>,
    pub(crate) rolled: bool,
}

impl ExecuteResult {
    /// 创建成功结果
    pub(crate) fn ok() -> Self {
        Self {
            err: None,
            skipped_errors: Vec::new(),
            rollback_errors: Vec::new(),
            rolled: false,
        }
    }

    /// 流程是否成功（无主错误）
    pub fn success(&self) -> bool {
        self.err.is_none()
    }

    /// 主错误（Strong 依赖失败时设置）
    pub fn err(&self) -> Option<&StepError> {
        self.err.as_ref()
    }

    /// 是否有弱依赖跳过的错误
    pub fn has_skipped_errors(&self) -> bool {
        !self.skipped_errors.is_empty()
    }

    /// 弱依赖跳过的错误列表
    pub fn skipped_errors(&self) -> &[StepError] {
        &self.skipped_errors
    }

    /// 是否有回滚错误
    pub fn has_rollback_errors(&self) -> bool {
        !self.rollback_errors.is_empty()
    }

    /// 回滚错误列表
    pub fn rollback_errors(&self) -> &[StepError] {
        &self.rollback_errors
    }

    /// 是否触发了回滚
    pub fn rolled(&self) -> bool {
        self.rolled
    }
}

impl fmt::Display for ExecuteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success() {
            write!(f, "flow succeeded")?;
        } else if let Some(e) = &self.err {
            write!(f, "flow failed: {e}")?;
        }
        if self.has_skipped_errors() {
            write!(f, ", {} skipped error(s)", self.skipped_errors.len())?;
        }
        if self.rolled {
            write!(f, ", rolled back")?;
            if self.has_rollback_errors() {
                write!(f, " with {} error(s)", self.rollback_errors.len())?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for ExecuteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecuteResult")
            .field("success", &self.success())
            .field("err", &self.err)
            .field("skipped_errors_count", &self.skipped_errors.len())
            .field("rollback_errors_count", &self.rollback_errors.len())
            .field("rolled", &self.rolled)
            .finish()
    }
}
