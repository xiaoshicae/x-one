//! Span KV 字段结构化存储 Layer
//!
//! 通过自定义 `tracing_subscriber::Layer`，在 Span 创建时将字段以
//! `Map<String, Value>` 形式存入 span extensions，供日志格式化器提取。

use serde_json::{Map, Value};
use tracing::Subscriber;
use tracing::span::{Attributes, Record};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use super::otel_fmt::JsonFieldVisitor;

/// Span 级别的 KV 字段存储
///
/// 以 `Map<String, Value>` 形式存储在 span extensions 中，
/// 供 `OtelJsonFormat` / `OtelConsoleFormat` 在格式化时提取。
///
/// # Examples
///
/// ```
/// use serde_json::{Map, Value};
/// use x_one::xlog::kv_layer::SpanKvFields;
///
/// let mut map = Map::new();
/// map.insert("user_id".to_string(), Value::String("123".to_string()));
/// let kv = SpanKvFields(map);
/// assert_eq!(kv.0.len(), 1);
/// ```
pub struct SpanKvFields(pub Map<String, Value>);

/// Span KV 字段收集 Layer
///
/// 在 span 创建时收集所有字段，在 `on_record` 时合并增量字段。
/// 字段以 `SpanKvFields` 形式存储在 span extensions 中。
pub struct KvLayer;

impl<S> Layer<S> for KvLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &tracing::span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut fields = Map::new();
            let mut visitor = JsonFieldVisitor::new(&mut fields);
            attrs.record(&mut visitor);
            span.extensions_mut().insert(SpanKvFields(fields));
        }
    }

    fn on_record(&self, id: &tracing::span::Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(kv) = extensions.get_mut::<SpanKvFields>() {
                let mut visitor = JsonFieldVisitor::new(&mut kv.0);
                values.record(&mut visitor);
            }
        }
    }
}
