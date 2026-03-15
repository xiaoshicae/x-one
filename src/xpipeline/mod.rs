//! xpipeline - 流式 channel 编排器
//!
//! 将多个 Processor 串联成 channel 链，每个 Processor 在独立 tokio task 中运行。
//!
//! # 核心概念
//!
//! - [`Frame`]：Pipeline 中传递的数据单元 trait
//! - [`Processor`]：处理器 trait，从 input 读取 Frame 处理后写入 output
//! - [`Pipeline`]：编排器，串联 Processor 链
//! - [`Monitor`]：监控回调接口
//! - [`RunResult`]：运行结果
//!
//! # Examples
//!
//! ```
//! use x_one::xpipeline::{Pipeline, Processor, Frame};
//! use tokio::sync::mpsc;
//!
//! // 自定义 Frame
//! struct TextFrame(String);
//! impl Frame for TextFrame {
//!     fn frame_type(&self) -> &str { "text" }
//! }
//! ```

mod config;
mod frame;
mod monitor;
mod pipeline;
mod processor;
mod result;

pub use config::PipelineConfig;
pub use frame::{EndFrame, ErrorFrame, Frame, MetadataFrame, StartFrame};
pub use monitor::{DefaultMonitor, Monitor, PipelineEvent, StepEvent};
pub use pipeline::Pipeline;
pub use processor::Processor;
pub use result::{RunResult, StepError};
