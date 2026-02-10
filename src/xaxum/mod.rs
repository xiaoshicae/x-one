//! xaxum - Axum HTTP 服务器模块
//!
//! 基于 Axum 封装，提供 HTTP 服务器构建器、日志和追踪中间件。
//!
//! ```ignore
//! use x_one::XAxum;
//! use axum::routing::get;
//!
//! let server = XAxum::new()
//!     .with_route_register(|r| r.route("/health", get(|| async { "ok" })))
//!     .build();
//! x_one::run_server(&server).await?;
//! ```

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
