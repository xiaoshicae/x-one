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

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;

    /// 辅助：构建带 KvLayer 的 subscriber，返回日志输出
    fn with_kv_subscriber<F>(f: F)
    where
        F: FnOnce(),
    {
        let _guard = tracing_subscriber::registry().with(KvLayer).set_default();
        f();
    }

    #[test]
    fn test_span_kv_fields_stored_in_extensions() {
        with_kv_subscriber(|| {
            let span = tracing::info_span!("test", user_id = "123", count = 42);
            let guard = span.enter();

            // 通过 tracing dispatcher 验证 span 存在并拥有字段
            tracing::dispatcher::get_default(|dispatch| {
                let id = span.id().unwrap();
                if let Some(registry) = dispatch.downcast_ref::<tracing_subscriber::Registry>() {
                    use tracing_subscriber::registry::LookupSpan;
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
                    use tracing_subscriber::registry::LookupSpan;
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
                    use tracing_subscriber::registry::LookupSpan;
                    let span_ref = registry.span(&id).unwrap();
                    let extensions = span_ref.extensions();
                    let kv = extensions.get::<SpanKvFields>().unwrap();
                    assert!(kv.0.is_empty(), "空 span 应该有空的 KvFields");
                }
            });

            drop(guard);
        });
    }
}
