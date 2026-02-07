//! 日志系统初始化

use super::config::{LogLevel, XLOG_CONFIG_KEY, XLogConfig};
use crate::{xconfig, xutil};
use std::path::Path;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

/// 初始化日志系统
pub fn init_xlog() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XLOG_CONFIG_KEY) {
        xutil::info_if_enable_debug("XLog config not found, skip init");
        return Ok(());
    }

    let c = get_config()?;
    xutil::info_if_enable_debug(&format!(
        "XOne initXLog got config: {}",
        xutil::to_json_string(&c)
    ));

    init_xlog_by_config(&c)
}

/// 根据配置初始化日志
fn init_xlog_by_config(c: &XLogConfig) -> Result<(), crate::error::XOneError> {
    // 确保日志目录存在
    if !xutil::dir_exist(&c.path) {
        std::fs::create_dir_all(&c.path).map_err(|e| {
            crate::error::XOneError::Log(format!(
                "create log dir failed, path=[{}], err=[{e}]",
                c.path
            ))
        })?;
    }

    // 创建文件 appender
    let log_file_path = Path::new(&c.path).join(format!("{}.log", c.name));
    let file_appender = tracing_appender::rolling::daily(&c.path, format!("{}.log", c.name));

    // 创建异步文件写入器
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 防止 guard 被 drop（需要保持活跃直到程序结束）
    // 通过 xhook 的 before_stop 来管理生命周期
    let guard = Box::new(guard);
    crate::xhook::before_stop(
        "xlog::flush",
        move || {
            drop(guard);
            Ok(())
        },
        crate::xhook::HookOptions::default(),
    );

    // 解析日志级别
    let level_filter = match c.level {
        LogLevel::Trace => "trace",
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    };

    // 构建 env filter
    let env_filter = EnvFilter::try_new(level_filter).unwrap_or_else(|_| EnvFilter::new("info"));

    // 构建 tracing subscriber
    // 文件输出：JSON 格式
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(true);

    // 控制台输出（如果启用）
    if c.console {
        if c.console_format_is_raw {
            // 原始 JSON 格式输出到控制台
            let console_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(std::io::stdout);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer)
                .with(console_layer)
                .try_init()
                .map_err(|e| {
                    crate::error::XOneError::Log(format!(
                        "init tracing subscriber failed, err=[{e}]"
                    ))
                })?;
        } else {
            // 带颜色的简洁格式输出到控制台
            let console_layer = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .with_writer(std::io::stdout);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer)
                .with(console_layer)
                .try_init()
                .map_err(|e| {
                    crate::error::XOneError::Log(format!(
                        "init tracing subscriber failed, err=[{e}]"
                    ))
                })?;
        }
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .try_init()
            .map_err(|e| {
                crate::error::XOneError::Log(format!("init tracing subscriber failed, err=[{e}]"))
            })?;
    }

    xutil::info_if_enable_debug(&format!(
        "XOne initXLog success, log file: {}",
        log_file_path.display()
    ));

    Ok(())
}

/// 从配置中获取日志配置
pub fn get_config() -> Result<XLogConfig, crate::error::XOneError> {
    Ok(xconfig::parse_config::<XLogConfig>(XLOG_CONFIG_KEY).unwrap_or_default())
}

