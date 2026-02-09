//! xaxum - Axum HTTP 服务器模块
//!
//! 对应 Go 版的 HTTP 服务器实现，提供 HTTP/HTTPS 服务器和 trace 中间件。

pub mod config;
pub mod server;
pub mod trace;

pub use config::{AxumConfig, AxumSwaggerConfig, load_config, load_swagger_config};
pub use server::{AxumServer, AxumTlsServer};
