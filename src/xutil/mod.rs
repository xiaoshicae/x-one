//! xutil - 通用工具模块
//!
//! 提供文件操作、JSON 序列化、环境变量、命令行解析、
//! 网络工具、重试机制、时长转换等基础工具函数。

pub mod cmd;
pub mod convert;
pub mod debug_log;
pub mod env;
pub mod file;
pub mod json;
pub mod net;
pub mod retry;
pub mod string;

// Re-export 常用 API，方便外部使用 xutil::xxx 调用
pub use cmd::{get_config_from_args, get_config_from_args_with};
pub use convert::to_duration;
pub use debug_log::{error_if_enable_debug, info_if_enable_debug, warn_if_enable_debug};
pub use env::{DEBUG_KEY, enable_debug};
pub use file::{dir_exist, file_exist};
pub use json::{to_json_string, to_json_string_indent};
pub use net::{extract_real_ip, get_local_ip};
pub use retry::{retry, retry_async};
pub use string::{default_if_empty, take_or_default};
