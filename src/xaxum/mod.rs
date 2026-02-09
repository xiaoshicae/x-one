//! xaxum - Axum HTTP 服务器模块
//!
//! 对应 Go 版的 HTTP 服务器实现，提供 HTTP/HTTPS 服务器和中间件。

pub mod banner;
pub mod config;
pub mod middleware;
pub mod options;
pub mod runner;
pub mod server;

pub use banner::print_banner;
pub use config::{AxumConfig, AxumSwaggerConfig, load_config, load_swagger_config};
pub use middleware::{log_middleware, trace_middleware};
pub use options::AxumOptions;
#[allow(deprecated)]
pub use runner::{run_axum, run_axum_tls, run_axum_with_options};
#[allow(deprecated)]
pub use server::{AxumServer, AxumTlsServer};
