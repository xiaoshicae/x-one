use opentelemetry::trace::{Tracer, TracerProvider};
use x_one::xlog::otel_fmt::*;

#[test]
fn test_get_otel_trace_ids_no_context_returns_empty() {
    let (trace_id, span_id) = get_otel_trace_ids();
    assert!(trace_id.is_empty(), "无活跃 Span 时 trace_id 应为空");
    assert!(span_id.is_empty(), "无活跃 Span 时 span_id 应为空");
}

#[test]
fn test_get_otel_trace_ids_with_active_span_returns_valid() {
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    let tracer = provider.tracer("test");

    tracer.in_span("test_span", |_cx| {
        let (trace_id, span_id) = get_otel_trace_ids();
        assert!(!trace_id.is_empty(), "活跃 Span 时 trace_id 不应为空");
        assert!(!span_id.is_empty(), "活跃 Span 时 span_id 不应为空");
        // trace_id 为 32 位十六进制字符串
        assert_eq!(trace_id.len(), 32, "trace_id 应为 32 字符");
        // span_id 为 16 位十六进制字符串
        assert_eq!(span_id.len(), 16, "span_id 应为 16 字符");
        // 不应全为 0（无效 ID）
        assert_ne!(
            trace_id, "00000000000000000000000000000000",
            "trace_id 不应全为 0"
        );
        assert_ne!(span_id, "0000000000000000", "span_id 不应全为 0");
    });
}

#[test]
fn test_get_otel_trace_ids_outside_span_returns_empty() {
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    let tracer = provider.tracer("test");

    // Span 内有 trace_id
    tracer.in_span("scoped_span", |_cx| {
        let (trace_id, _) = get_otel_trace_ids();
        assert!(!trace_id.is_empty());
    });

    // Span 结束后应恢复为空
    let (trace_id, span_id) = get_otel_trace_ids();
    assert!(trace_id.is_empty(), "Span 结束后 trace_id 应为空");
    assert!(span_id.is_empty(), "Span 结束后 span_id 应为空");
}

#[test]
fn test_otel_json_format_produces_valid_json() {
    use tracing_subscriber::prelude::*;

    // 使用内存 buffer 捕获输出
    let buf = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
    let buf_clone = buf.clone();

    let writer =
        move || -> Box<dyn std::io::Write + Send> { Box::new(SharedWriter(buf_clone.clone())) };

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(user_id = 42, "hello world");
    });

    let output = {
        let guard = buf.lock();
        String::from_utf8(guard.clone()).unwrap()
    };

    // 解析 JSON 验证
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(json["level"], "INFO");
    assert!(json["timestamp"].as_str().is_some(), "应包含 timestamp");
    assert!(json["target"].as_str().is_some(), "应包含 target");
    assert!(json["threadName"].as_str().is_some(), "应包含 threadName");
    // caller 格式为 "file:line"
    let caller = json["caller"].as_str().expect("应包含 caller");
    assert!(caller.contains(':'), "caller 应为 file:line 格式");
    assert_eq!(json["fields"]["message"], "hello world");
    assert_eq!(json["fields"]["user_id"], 42);
    // 无活跃 Span 时不应包含 trace_id
    assert!(
        json.get("trace_id").is_none(),
        "无 Span 时不应包含 trace_id"
    );
}

#[test]
fn test_otel_json_format_includes_trace_id_with_active_span() {
    use tracing_subscriber::prelude::*;

    let buf = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
    let buf_clone = buf.clone();

    let writer =
        move || -> Box<dyn std::io::Write + Send> { Box::new(SharedWriter(buf_clone.clone())) };

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer),
    );

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    let tracer = provider.tracer("test");

    tracing::subscriber::with_default(subscriber, || {
        tracer.in_span("request", |_cx| {
            tracing::info!("processing");
        });
    });

    let output = {
        let guard = buf.lock();
        String::from_utf8(guard.clone()).unwrap()
    };

    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(json["level"], "INFO");
    assert_eq!(json["fields"]["message"], "processing");
    // 有活跃 Span 时应包含 trace_id 和 span_id
    let trace_id = json["trace_id"].as_str().unwrap();
    let span_id = json["span_id"].as_str().unwrap();
    assert_eq!(trace_id.len(), 32, "trace_id 应为 32 字符");
    assert_eq!(span_id.len(), 16, "span_id 应为 16 字符");
}

#[test]
fn test_otel_console_format_without_trace_id() {
    use tracing_subscriber::prelude::*;

    let buf = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
    let buf_clone = buf.clone();

    let writer =
        move || -> Box<dyn std::io::Write + Send> { Box::new(SharedWriter(buf_clone.clone())) };

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelConsoleFormat)
            .with_writer(writer),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("test message");
    });

    let output = {
        let guard = buf.lock();
        String::from_utf8(guard.clone()).unwrap()
    };

    assert!(output.contains("INFO"), "应包含级别");
    assert!(output.contains("test message"), "应包含消息内容");
    // 应包含 caller 信息（文件名:行号）
    assert!(output.contains("otel_fmt.rs:"), "应包含调用位置");
}

#[test]
fn test_otel_console_format_with_trace_id() {
    use tracing_subscriber::prelude::*;

    let buf = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
    let buf_clone = buf.clone();

    let writer =
        move || -> Box<dyn std::io::Write + Send> { Box::new(SharedWriter(buf_clone.clone())) };

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelConsoleFormat)
            .with_writer(writer),
    );

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    let tracer = provider.tracer("test");

    tracing::subscriber::with_default(subscriber, || {
        tracer.in_span("request", |_cx| {
            tracing::error!("something failed");
        });
    });

    let output = {
        let guard = buf.lock();
        String::from_utf8(guard.clone()).unwrap()
    };

    assert!(output.contains("ERROR"), "应包含级别");
    assert!(output.contains("something failed"), "应包含消息内容");
    // trace_id 应在输出中（32 位十六进制）
    // 去掉 ANSI 颜色码后检查
    let stripped = strip_ansi(&output);
    // 提取并验证 trace_id 格式
    assert!(
        stripped.chars().any(|c| c.is_ascii_hexdigit()),
        "应包含 trace_id 十六进制字符"
    );
}

#[test]
fn test_otel_console_format_includes_extra_fields() {
    use tracing_subscriber::prelude::*;

    let buf = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
    let buf_clone = buf.clone();

    let writer =
        move || -> Box<dyn std::io::Write + Send> { Box::new(SharedWriter(buf_clone.clone())) };

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelConsoleFormat)
            .with_writer(writer),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(user_id = 123, action = "login", "user event");
    });

    let output = {
        let guard = buf.lock();
        String::from_utf8(guard.clone()).unwrap()
    };

    assert!(output.contains("user event"), "应包含消息");
    assert!(output.contains("user_id"), "应包含额外字段 user_id");
    assert!(output.contains("123"), "应包含字段值");
    assert!(output.contains("action"), "应包含额外字段 action");
    assert!(output.contains("login"), "应包含字段值 login");
}

// ---- 辅助工具 ----

/// 共享写入器，用于捕获 tracing 输出到内存
struct SharedWriter(std::sync::Arc<parking_lot::Mutex<Vec<u8>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 去除 ANSI 转义序列
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
