//! x-one: Rust 三方库集成框架
//!
//! xone 框架，提供配置管理、日志、Hook 生命周期、
//! HTTP 服务、链路追踪、数据库连接管理、本地缓存等功能。

pub mod error;
pub mod xaxum;
pub mod xcache;
pub mod xconfig;
pub mod xflow;
pub mod xhook;
pub mod xhttp;
pub mod xlog;
pub mod xorm;
pub mod xserver;
pub mod xtrace;
pub mod xutil;

pub use error::{Result, XOneError};
pub use xaxum::{XAxum, XAxumServer};
pub use xserver::Server;
pub use xserver::blocking::BlockingServer;
pub use xserver::{init, run_blocking_server, run_server, shutdown};
