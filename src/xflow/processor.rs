//! 处理器 trait 和依赖类型定义
//!
//! 定义流程步骤的核心抽象 `Processor` trait 和依赖强度 `Dependency` 枚举。

use std::fmt;

use crate::XOneError;

/// 步骤依赖强度
///
/// 控制步骤失败时的流程行为：
/// - `Strong`：失败则中断流程并触发回滚
/// - `Weak`：失败则跳过并继续执行后续步骤
///
/// # Examples
///
/// ```
/// use x_one::xflow::Dependency;
///
/// let dep = Dependency::Strong;
/// assert_eq!(dep, Dependency::Strong);
/// assert_eq!(dep.to_string(), "strong");
///
/// let weak = Dependency::Weak;
/// assert_eq!(weak.to_string(), "weak");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dependency {
    /// 强依赖：步骤失败会中断流程并触发回滚
    Strong,
    /// 弱依赖：步骤失败只记录错误，流程继续
    Weak,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dependency::Strong => write!(f, "strong"),
            Dependency::Weak => write!(f, "weak"),
        }
    }
}

/// 流程处理器 trait
///
/// 定义流程步骤的四个核心方法。实现者需要至少实现 `name()` 和 `process()`,
/// `dependency()` 默认返回 `Strong`，`rollback()` 默认空操作。
///
/// # Examples
///
/// ```
/// use x_one::xflow::{Processor, Dependency};
/// use x_one::XOneError;
///
/// struct MyStep;
///
/// impl Processor<String> for MyStep {
///     fn name(&self) -> &str { "my_step" }
///
///     fn process(&self, data: &mut String) -> Result<(), XOneError> {
///         data.push_str(" processed");
///         Ok(())
///     }
/// }
///
/// let step = MyStep;
/// assert_eq!(step.name(), "my_step");
/// assert_eq!(step.dependency(), Dependency::Strong);
///
/// let mut data = "hello".to_string();
/// step.process(&mut data).unwrap();
/// assert_eq!(data, "hello processed");
/// ```
pub trait Processor<T>: Send + Sync {
    /// 处理器名称，用于日志和错误追踪
    fn name(&self) -> &str;

    /// 依赖强度，默认 `Strong`
    fn dependency(&self) -> Dependency {
        Dependency::Strong
    }

    /// 执行处理逻辑
    fn process(&self, data: &mut T) -> Result<(), XOneError>;

    /// 回滚逻辑，默认空操作
    fn rollback(&self, data: &mut T) -> Result<(), XOneError> {
        let _ = data;
        Ok(())
    }
}
