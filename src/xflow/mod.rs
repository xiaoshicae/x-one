//! 流程编排框架
//!
//! 提供顺序执行处理器、强/弱依赖模型和自动逆序回滚能力。
//!
//! # 核心概念
//!
//! - [`Processor`]：处理器 trait，定义步骤的处理和回滚逻辑
//! - [`Step`]：闭包式处理器，通过 Builder 模式快速创建步骤
//! - [`Flow`]：编排器，顺序执行处理器并管理回滚
//! - [`Dependency`]：依赖强度（Strong/Weak）
//! - [`ExecuteResult`]：流程执行结果
//! - [`Monitor`]：监控回调接口
//!
//! # Examples
//!
//! ```
//! use x_one::xflow::{Flow, Step};
//!
//! let flow = Flow::new("example")
//!     .step(Step::new("step1").process(|data: &mut i32| {
//!         *data += 10;
//!         Ok(())
//!     }))
//!     .step(Step::new("step2").process(|data: &mut i32| {
//!         *data *= 2;
//!         Ok(())
//!     }));
//!
//! let mut data = 5;
//! let result = flow.execute(&mut data);
//! assert!(result.success());
//! assert_eq!(data, 30); // (5 + 10) * 2
//! ```

mod flow;
mod monitor;
mod processor;
mod result;
mod step;

pub use flow::Flow;
pub use monitor::{DefaultMonitor, Monitor};
pub use processor::{Dependency, Processor};
pub use result::{ExecuteResult, StepError};
pub use step::Step;
