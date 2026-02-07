pub mod config;
pub mod init;
pub mod client;

pub use client::{c, delete, get, head, patch, post, put};
pub use config::XHttpConfig;
pub use init::{build_client, init_xhttp};

pub fn register_hook() {
    crate::xhook::before_start(
        "xhttp::init",
        init::init_xhttp,
        crate::xhook::HookOptions {
            order: 4,
            ..Default::default()
        },
    );
}
