//! xaxum - Axum HTTP 服务器模块
//!
//! 对应 Go 版的 HTTP 服务器实现，提供 HTTP 服务器和中间件。

pub mod banner;
pub mod builder;
pub mod config;
pub mod middleware;
pub mod server;

pub use banner::print_banner;
pub use builder::XAxum;
pub use config::{AxumConfig, AxumSwaggerConfig};
pub use middleware::{log_middleware, trace_middleware};
pub use server::XAxumServer;
