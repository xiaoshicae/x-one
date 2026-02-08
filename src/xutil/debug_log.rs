//! 框架内部 debug 日志工具
//!
//! 这些函数仅在 `SERVER_ENABLE_DEBUG=true` 时输出日志到控制台，
//! 用于 x-one 框架启动过程中的调试信息。

use super::env::enable_debug;

/// debug 模式下输出 ERROR 级别日志
pub fn error_if_enable_debug(msg: &str) {
    log_if_enable_debug("ERROR", msg);
}

/// debug 模式下输出 INFO 级别日志
pub fn info_if_enable_debug(msg: &str) {
    log_if_enable_debug("INFO", msg);
}

/// debug 模式下输出 WARN 级别日志
pub fn warn_if_enable_debug(msg: &str) {
    log_if_enable_debug("WARN", msg);
}

/// debug 模式下输出指定级别日志
pub fn log_if_enable_debug(level: &str, msg: &str) {
    if enable_debug() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let color = match level {
            "ERROR" => "\x1b[31m", // 红色
            "WARN" => "\x1b[33m",  // 黄色
            "INFO" => "\x1b[36m",  // 青色
            "DEBUG" => "\x1b[37m", // 灰色
            _ => "\x1b[0m",
        };
        eprintln!("{color}{level}\x1b[0m[{now}] {msg}");
    }
}
