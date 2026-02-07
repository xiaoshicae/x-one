pub mod config;
pub mod init;
pub mod client;

pub use client::{c, delete, get, head, patch, post, put};
pub use config::XHttpConfig;
pub use init::{build_client, init_xhttp};

pub fn register_hook() {
    crate::before_start!(init::init_xhttp, crate::xhook::HookOptions::with_order(4));
}
