//! KvLayer 集成测试

use std::sync::{Arc, Mutex};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use x_one::xlog::kv_layer::{KvLayer, SpanKvFields};
use x_one::xlog::otel_fmt::OtelJsonFormat;

/// 捕获日志输出的 Writer
#[derive(Clone)]
struct CaptureWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl CaptureWriter {
    fn new() -> Self {
        Self {
            buf: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn output(&self) -> String {
        let buf = self.buf.lock().unwrap();
        String::from_utf8_lossy(&buf).to_string()
    }
}

impl std::io::Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// 辅助：构建带 KvLayer 的 subscriber，返回日志输出
fn with_kv_subscriber<F>(f: F)
where
    F: FnOnce(),
{
    let _guard = tracing_subscriber::registry().with(KvLayer).set_default();
    f();
}

// ============================================================
// KvLayer + SpanKvFields 底层 API 测试
// ============================================================

#[test]
fn test_span_kv_fields_stored_in_extensions() {
    with_kv_subscriber(|| {
        let span = tracing::info_span!("test", user_id = "123", count = 42);
        let guard = span.enter();

        tracing::dispatcher::get_default(|dispatch| {
            let id = span.id().unwrap();
            if let Some(registry) = dispatch.downcast_ref::<tracing_subscriber::Registry>() {
                let span_ref = registry.span(&id).unwrap();
                let extensions = span_ref.extensions();
                let kv = extensions.get::<SpanKvFields>().unwrap();
                assert_eq!(kv.0.get("user_id").unwrap(), "123");
                assert_eq!(kv.0.get("count").unwrap(), 42);
            }
        });

        drop(guard);
    });
}

#[test]
fn test_span_kv_fields_on_record_merges() {
    with_kv_subscriber(|| {
        let span = tracing::info_span!("test", user_id = tracing::field::Empty, count = 1);
        let guard = span.enter();

        span.record("user_id", "merged_value");

        tracing::dispatcher::get_default(|dispatch| {
            let id = span.id().unwrap();
            if let Some(registry) = dispatch.downcast_ref::<tracing_subscriber::Registry>() {
                let span_ref = registry.span(&id).unwrap();
                let extensions = span_ref.extensions();
                let kv = extensions.get::<SpanKvFields>().unwrap();
                assert_eq!(kv.0.get("user_id").unwrap(), "merged_value");
                assert_eq!(kv.0.get("count").unwrap(), 1);
            }
        });

        drop(guard);
    });
}

#[test]
fn test_kv_layer_empty_span_no_fields() {
    with_kv_subscriber(|| {
        let span = tracing::info_span!("empty");
        let guard = span.enter();

        tracing::dispatcher::get_default(|dispatch| {
            let id = span.id().unwrap();
            if let Some(registry) = dispatch.downcast_ref::<tracing_subscriber::Registry>() {
                let span_ref = registry.span(&id).unwrap();
                let extensions = span_ref.extensions();
                let kv = extensions.get::<SpanKvFields>().unwrap();
                assert!(kv.0.is_empty(), "空 span 应该有空的 KvFields");
            }
        });

        drop(guard);
    });
}

// ============================================================
// KvLayer + OtelJsonFormat 端到端测试
// ============================================================

#[test]
fn test_span_kv_injected_into_json_output() {
    let writer = CaptureWriter::new();
    let writer_clone = writer.clone();

    let subscriber = tracing_subscriber::registry().with(KvLayer).with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer_clone),
    );

    let _guard = tracing::subscriber::set_default(subscriber);

    let _span_guard = tracing::info_span!("", user_id = "u123", request_id = "r456").entered();
    tracing::info!("hello");

    let output = writer.output();
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

    let fields = json.get("fields").unwrap().as_object().unwrap();
    assert_eq!(fields.get("user_id").unwrap(), "u123");
    assert_eq!(fields.get("request_id").unwrap(), "r456");
    assert_eq!(fields.get("message").unwrap(), "hello");
}

#[test]
fn test_event_fields_override_span_kv() {
    let writer = CaptureWriter::new();
    let writer_clone = writer.clone();

    let subscriber = tracing_subscriber::registry().with(KvLayer).with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer_clone),
    );

    let _guard = tracing::subscriber::set_default(subscriber);

    let _span_guard = tracing::info_span!("", action = "span_action").entered();
    tracing::info!(action = "event_action", "test override");

    let output = writer.output();
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

    let fields = json.get("fields").unwrap().as_object().unwrap();
    // event 字段应覆盖 span 同名字段
    assert_eq!(fields.get("action").unwrap(), "event_action");
}

#[test]
fn test_nested_spans_inner_overrides_outer() {
    let writer = CaptureWriter::new();
    let writer_clone = writer.clone();

    let subscriber = tracing_subscriber::registry().with(KvLayer).with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer_clone),
    );

    let _guard = tracing::subscriber::set_default(subscriber);

    let _outer = tracing::info_span!("", env = "outer", outer_only = "yes").entered();
    let _inner = tracing::info_span!("", env = "inner", inner_only = "yes").entered();
    tracing::info!("nested test");

    let output = writer.output();
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

    let fields = json.get("fields").unwrap().as_object().unwrap();
    // 内层覆盖外层同名 key
    assert_eq!(fields.get("env").unwrap(), "inner");
    // 各层独有字段都保留
    assert_eq!(fields.get("outer_only").unwrap(), "yes");
    assert_eq!(fields.get("inner_only").unwrap(), "yes");
}

#[test]
fn test_no_span_no_extra_fields() {
    let writer = CaptureWriter::new();
    let writer_clone = writer.clone();

    let subscriber = tracing_subscriber::registry().with(KvLayer).with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer_clone),
    );

    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!(key = "val", "no span");

    let output = writer.output();
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

    let fields = json.get("fields").unwrap().as_object().unwrap();
    assert_eq!(fields.get("key").unwrap(), "val");
    assert_eq!(fields.get("message").unwrap(), "no span");
    // 只有 event 字段，没有额外的 span KV
    assert_eq!(fields.len(), 2);
}

#[test]
fn test_xlog_kv_macro() {
    let writer = CaptureWriter::new();
    let writer_clone = writer.clone();

    let subscriber = tracing_subscriber::registry().with(KvLayer).with(
        tracing_subscriber::fmt::layer()
            .event_format(OtelJsonFormat)
            .with_writer(writer_clone),
    );

    let _guard = tracing::subscriber::set_default(subscriber);

    let _kv = x_one::xlog_kv!(user_id = "macro_user", trace = true);
    tracing::info!("macro test");

    let output = writer.output();
    let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

    let fields = json.get("fields").unwrap().as_object().unwrap();
    assert_eq!(fields.get("user_id").unwrap(), "macro_user");
    assert_eq!(fields.get("trace").unwrap(), true);
    assert_eq!(fields.get("message").unwrap(), "macro test");
}
