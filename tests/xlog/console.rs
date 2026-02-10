use x_one::xlog::console::*;

#[test]
fn test_get_level_color_error() {
    assert_eq!(get_level_color(&tracing::Level::ERROR), COLOR_RED);
}

#[test]
fn test_get_level_color_warn() {
    assert_eq!(get_level_color(&tracing::Level::WARN), COLOR_YELLOW);
}

#[test]
fn test_get_level_color_info() {
    assert_eq!(get_level_color(&tracing::Level::INFO), COLOR_BLUE);
}

#[test]
fn test_get_level_color_debug() {
    assert_eq!(get_level_color(&tracing::Level::DEBUG), COLOR_GRAY);
}

#[test]
fn test_format_console_line_without_trace() {
    let line = format_console_line(
        &tracing::Level::INFO,
        "2024-01-01 00:00:00.000",
        "hello",
        "",
        "src/main.rs:42",
    );
    assert!(line.contains("INFO"));
    assert!(line.contains("hello"));
    assert!(line.contains("2024-01-01"));
    assert!(line.contains("src/main.rs:42"));
}

#[test]
fn test_format_console_line_with_trace() {
    let line = format_console_line(
        &tracing::Level::ERROR,
        "2024-01-01 00:00:00.000",
        "error msg",
        "trace-123",
        "src/handler.rs:10",
    );
    assert!(line.contains("ERROR"));
    assert!(line.contains("error msg"));
    assert!(line.contains("trace-123"));
    assert!(line.contains("src/handler.rs:10"));
}

#[test]
fn test_format_console_line_without_caller() {
    let line = format_console_line(
        &tracing::Level::WARN,
        "2024-01-01 00:00:00.000",
        "warn msg",
        "",
        "",
    );
    assert!(line.contains("WARN"));
    assert!(line.contains("warn msg"));
    // 空 caller 不应出现括号
    assert!(!line.contains("()"));
}
