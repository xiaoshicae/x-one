//! x-one: Rust 三方库集成框架
//!
//! xone 框架，提供配置管理、日志、Hook 生命周期、
//! HTTP 服务、链路追踪、数据库连接管理、本地缓存等功能。

pub mod error;
pub mod xconfig;
pub mod xhook;
pub mod xserver;
pub mod xutil;

#[cfg(feature = "log")]
pub mod xlog;
#[cfg(feature = "trace")]
pub mod xtrace;
#[cfg(feature = "http")]
pub mod xhttp;
#[cfg(feature = "orm")]
pub mod xorm;
#[cfg(feature = "cache")]
pub mod xcache;
#[cfg(feature = "axum-server")]
pub mod xaxum;
#[cfg(feature = "redis-store")]
pub mod xredis;
#[cfg(feature = "metric")]
pub mod xmetric;
#[cfg(feature = "flow")]
pub mod xflow;
#[cfg(feature = "pipeline")]
pub mod xpipeline;

pub use error::{Result, XOneError};
#[cfg(feature = "axum-server")]
pub use xaxum::{XAxum, XAxumServer};
pub use xserver::Server;
pub use xserver::blocking::BlockingServer;
pub use xserver::{init, run_blocking_server, run_server, shutdown};
