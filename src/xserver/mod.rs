//! Server trait 定义和服务运行逻辑

pub mod blocking;
pub mod runner;
pub mod server;

pub use runner::{
    ensure_init, invoke_before_stop_hooks_safe, run_blocking_server, run_server, run_with_server,
};
pub use server::Server;
