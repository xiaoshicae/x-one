//! Pipeline 执行结果

use std::fmt;

use crate::XOneError;

/// 处理器执行错误
#[derive(Debug)]
pub struct StepError {
    /// 处理器名称
    pub processor_name: String,
    /// 错误详情
    pub err: XOneError,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "processor=[{}], err=[{}]", self.processor_name, self.err)
    }
}

impl std::error::Error for StepError {}

/// Pipeline 运行结果
#[derive(Debug, Default)]
pub struct RunResult {
    /// 所有处理器的错误
    pub errors: Vec<StepError>,
}

impl RunResult {
    /// 所有处理器是否成功完成
    pub fn success(&self) -> bool {
        self.errors.is_empty()
    }

    /// 是否存在处理器错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl fmt::Display for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            return Ok(());
        }
        write!(f, "pipeline failed: errors=[{}]", self.errors.len())
    }
}
