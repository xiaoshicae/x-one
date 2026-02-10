//! OpenTelemetry trace 上下文日志格式化器
//!
//! 为 JSON 和控制台日志输出自动注入 `trace_id` 和 `span_id`。
//! 当存在活跃的 OpenTelemetry Span 时自动提取，否则省略。

use std::fmt;

use opentelemetry::trace::TraceContextExt;
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::{self, FormatFields};
use tracing_subscriber::fmt::{FmtContext, FormatEvent};
use tracing_subscriber::registry::LookupSpan;

/// 从当前 OpenTelemetry 上下文提取 trace_id 和 span_id
///
/// 若无活跃 Span 或上下文无效，返回空字符串。
///
/// # Examples
///
/// ```
/// let (trace_id, span_id) = x_one::xlog::otel_fmt::get_otel_trace_ids();
/// // 无活跃 Span 时返回空字符串
/// assert!(trace_id.is_empty());
/// ```
pub fn get_otel_trace_ids() -> (String, String) {
    let ctx = opentelemetry::Context::current();
    let span_ref = ctx.span();
    let sc = span_ref.span_context();
    if sc.is_valid() {
        (sc.trace_id().to_string(), sc.span_id().to_string())
    } else {
        (String::new(), String::new())
    }
}

/// JSON 字段收集 Visitor
///
/// 将 tracing 事件/span 的字段收集到 `Map<String, Value>` 中。
pub(crate) struct JsonFieldVisitor<'a> {
    fields: &'a mut Map<String, Value>,
}

impl<'a> JsonFieldVisitor<'a> {
    /// 创建新的字段收集器
    pub(crate) fn new(fields: &'a mut Map<String, Value>) -> Self {
        Self { fields }
    }
}

impl<'a> Visit for JsonFieldVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::json!(format!("{value:?}")),
        );
    }
}

/// 带 OpenTelemetry trace 上下文的 JSON 格式化器
///
/// 在标准 JSON 日志字段基础上，自动注入 `trace_id` 和 `span_id`。
/// 输出格式与 `tracing-subscriber` 内置 JSON 格式兼容，
/// 额外增加 `trace_id`、`span_id` 顶层字段。
pub struct OtelJsonFormat;

impl<S, N> FormatEvent<S, N> for OtelJsonFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let mut root = Map::new();

        // 时间戳
        let now = chrono::Utc::now();
        root.insert(
            "timestamp".to_string(),
            serde_json::json!(now.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        );

        // 日志级别
        root.insert(
            "level".to_string(),
            serde_json::json!(meta.level().as_str()),
        );

        // OpenTelemetry trace 上下文
        let (trace_id, span_id) = get_otel_trace_ids();
        if !trace_id.is_empty() {
            root.insert("trace_id".to_string(), serde_json::json!(trace_id));
            root.insert("span_id".to_string(), serde_json::json!(span_id));
        }

        // 事件字段（包括 message）
        // 先从 span extensions 提取 KV（外层 → 内层，后者覆盖前者同名 key）
        let mut fields = Map::new();
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                let extensions = span.extensions();
                if let Some(kv) = extensions.get::<super::kv_layer::SpanKvFields>() {
                    for (k, v) in &kv.0 {
                        fields.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        // event 字段最后写入，覆盖同名 span 字段
        let mut visitor = JsonFieldVisitor::new(&mut fields);
        event.record(&mut visitor);
        root.insert("fields".to_string(), Value::Object(fields));

        // 调用位置（文件:行号）
        if let (Some(file), Some(line)) = (meta.file(), meta.line()) {
            root.insert(
                "caller".to_string(),
                serde_json::json!(format!("{file}:{line}")),
            );
        }

        // 目标模块
        root.insert("target".to_string(), serde_json::json!(meta.target()));

        // 线程名
        let thread = std::thread::current();
        root.insert(
            "threadName".to_string(),
            serde_json::json!(thread.name().unwrap_or("unknown")),
        );

        // Span 上下文链
        if let Some(scope) = ctx.event_scope() {
            let spans: Vec<Value> = scope
                .map(|span| serde_json::json!({"name": span.name()}))
                .collect();
            if !spans.is_empty() {
                root.insert("spans".to_string(), Value::Array(spans));
            }
        }

        let json_str = serde_json::to_string(&root).map_err(|_| fmt::Error)?;
        writeln!(writer, "{json_str}")
    }
}

/// 带 OpenTelemetry trace 上下文的控制台格式化器
///
/// 以彩色文本格式输出日志，自动在时间戳和消息之间插入 `trace_id`。
pub struct OtelConsoleFormat;

impl<S, N> FormatEvent<S, N> for OtelConsoleFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();

        // 先从 span extensions 提取 KV
        let mut fields = Map::new();
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                let extensions = span.extensions();
                if let Some(kv) = extensions.get::<super::kv_layer::SpanKvFields>() {
                    for (k, v) in &kv.0 {
                        fields.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        // event 字段最后写入，覆盖同名 span 字段
        let mut visitor = JsonFieldVisitor::new(&mut fields);
        event.record(&mut visitor);

        // 提取 message
        let message = fields
            .remove("message")
            .and_then(|v| match v {
                Value::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or_default();

        // 拼接额外字段到消息末尾
        let extras: Vec<String> = fields
            .iter()
            .map(|(k, v)| match v {
                Value::String(s) => format!("{k}={s}"),
                other => format!("{k}={other}"),
            })
            .collect();

        let full_message = if extras.is_empty() {
            message
        } else {
            format!("{message} {}", extras.join(" "))
        };

        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        let (trace_id, _) = get_otel_trace_ids();

        let caller = match (meta.file(), meta.line()) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            _ => String::new(),
        };

        let line = super::console::format_console_line(
            meta.level(),
            &timestamp,
            &full_message,
            &trace_id,
            &caller,
        );
        write!(writer, "{line}")
    }
}
