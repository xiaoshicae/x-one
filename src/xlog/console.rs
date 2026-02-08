//! 控制台着色输出

/// ANSI 颜色代码
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_BLUE: &str = "\x1b[36m";
pub const COLOR_GRAY: &str = "\x1b[37m";
pub const COLOR_RESET: &str = "\x1b[0m";

/// 根据日志级别获取控制台颜色
pub fn get_level_color(level: &tracing::Level) -> &'static str {
    match *level {
        tracing::Level::ERROR => COLOR_RED,
        tracing::Level::WARN => COLOR_YELLOW,
        tracing::Level::INFO => COLOR_BLUE,
        tracing::Level::DEBUG | tracing::Level::TRACE => COLOR_GRAY,
    }
}

/// 格式化控制台日志行
pub fn format_console_line(
    level: &tracing::Level,
    timestamp: &str,
    message: &str,
    trace_id: &str,
) -> String {
    let color = get_level_color(level);
    let level_text = level.as_str().to_uppercase();

    if trace_id.is_empty() {
        format!("{color}{level_text}{COLOR_RESET}[{timestamp}] {message}\n")
    } else {
        format!(
            "{color}{level_text}{COLOR_RESET}[{timestamp}] {COLOR_BLUE}{trace_id}{COLOR_RESET} {message}\n"
        )
    }
}
