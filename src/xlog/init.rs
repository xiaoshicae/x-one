//! 日志系统初始化

use super::config::{LogLevel, XLOG_CONFIG_KEY, XLogConfig};
use super::otel_fmt::{OtelConsoleFormat, OtelJsonFormat};
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

    let c = super::client::get_config();
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
    // order=100 确保在其他 hook（xtrace=1, xcache=2, xorm=3）之后执行，
    // 使其他 hook 的 stop 过程仍可使用日志系统
    let guard = Box::new(guard);
    crate::before_stop!(
        move || {
            drop(guard);
            Ok(())
        },
        crate::xhook::HookOptions::with_order(100)
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
    let env_filter = EnvFilter::try_new(level_filter).unwrap_or_else(|e| {
        xutil::warn_if_enable_debug(&format!(
            "XLog invalid level filter [{level_filter}], fallback to info, err=[{e}]"
        ));
        EnvFilter::new("info")
    });

    // 文件输出：JSON 格式（自动注入 trace_id）
    let file_layer = tracing_subscriber::fmt::layer()
        .event_format(OtelJsonFormat)
        .with_writer(non_blocking);

    fn init_err(e: impl std::fmt::Display) -> crate::error::XOneError {
        crate::error::XOneError::Log(format!("init tracing subscriber failed, err=[{e}]"))
    }

    // Span KV 字段收集 Layer
    let kv_layer = super::kv_layer::KvLayer;

    // 组装 subscriber
    if c.console {
        if c.console_format_is_raw {
            // 原始 JSON 格式输出到控制台（自动注入 trace_id）
            let console_layer = tracing_subscriber::fmt::layer()
                .event_format(OtelJsonFormat)
                .with_writer(std::io::stdout);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(kv_layer)
                .with(file_layer)
                .with(console_layer)
                .try_init()
                .map_err(init_err)?;
        } else {
            // 带颜色的简洁格式输出到控制台（自动注入 trace_id）
            let console_layer = tracing_subscriber::fmt::layer()
                .event_format(OtelConsoleFormat)
                .with_writer(std::io::stdout);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(kv_layer)
                .with(file_layer)
                .with(console_layer)
                .try_init()
                .map_err(init_err)?;
        }
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(kv_layer)
            .with(file_layer)
            .try_init()
            .map_err(init_err)?;
    }

    xutil::info_if_enable_debug(&format!(
        "XOne initXLog success, log file: {}",
        log_file_path.display()
    ));

    Ok(())
}
