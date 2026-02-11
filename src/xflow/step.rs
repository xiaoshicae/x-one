//! 闭包式处理器
//!
//! `Step<T>` 通过 Builder 模式用闭包定义处理和回滚逻辑，适合简单场景。
//! 复杂场景可直接实现 `Processor` trait。

use crate::XOneError;

use super::processor::{Dependency, Processor};

type BoxFn<T> = Box<dyn Fn(&mut T) -> Result<(), XOneError> + Send + Sync>;

/// 闭包式流程步骤
///
/// 通过链式调用定义处理逻辑和回滚逻辑，无需实现 trait。
///
/// # Examples
///
/// ```
/// use x_one::xflow::{Step, Processor, Dependency};
///
/// let step = Step::new("greet")
///     .process(|data: &mut String| {
///         data.push_str(" world");
///         Ok(())
///     })
///     .rollback(|data: &mut String| {
///         *data = "rolled back".to_string();
///         Ok(())
///     });
///
/// assert_eq!(step.name(), "greet");
/// assert_eq!(step.dependency(), Dependency::Strong);
///
/// let mut data = "hello".to_string();
/// Processor::process(&step, &mut data).unwrap();
/// assert_eq!(data, "hello world");
/// ```
pub struct Step<T> {
    name: String,
    dependency: Dependency,
    process_fn: Option<BoxFn<T>>,
    rollback_fn: Option<BoxFn<T>>,
}

impl<T> Step<T> {
    /// 创建强依赖步骤
    ///
    /// # Examples
    ///
    /// ```
    /// use x_one::xflow::{Step, Processor, Dependency};
    ///
    /// let step = Step::<()>::new("init");
    /// assert_eq!(step.name(), "init");
    /// assert_eq!(step.dependency(), Dependency::Strong);
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependency: Dependency::Strong,
            process_fn: None,
            rollback_fn: None,
        }
    }

    /// 创建弱依赖步骤
    ///
    /// # Examples
    ///
    /// ```
    /// use x_one::xflow::{Step, Processor, Dependency};
    ///
    /// let step = Step::<()>::weak("optional");
    /// assert_eq!(step.name(), "optional");
    /// assert_eq!(step.dependency(), Dependency::Weak);
    /// ```
    pub fn weak(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependency: Dependency::Weak,
            process_fn: None,
            rollback_fn: None,
        }
    }

    /// 设置处理逻辑
    pub fn process<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Result<(), XOneError> + Send + Sync + 'static,
    {
        self.process_fn = Some(Box::new(f));
        self
    }

    /// 设置回滚逻辑
    pub fn rollback<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Result<(), XOneError> + Send + Sync + 'static,
    {
        self.rollback_fn = Some(Box::new(f));
        self
    }
}

impl<T: Send + Sync> Processor<T> for Step<T> {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependency(&self) -> Dependency {
        self.dependency
    }

    fn process(&self, data: &mut T) -> Result<(), XOneError> {
        match &self.process_fn {
            Some(f) => f(data),
            None => Ok(()),
        }
    }

    fn rollback(&self, data: &mut T) -> Result<(), XOneError> {
        match &self.rollback_fn {
            Some(f) => f(data),
            None => Ok(()),
        }
    }
}
