//! xtrace 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;
use opentelemetry::global;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::config::{XTRACE_CONFIG_KEY, XTraceConfig};

/// 全局 trace 启用标志
static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

/// 全局 TracerProvider（需要在 shutdown 时用到）
static PROVIDER: std::sync::OnceLock<Mutex<Option<SdkTracerProvider>>> = std::sync::OnceLock::new();

fn provider_store() -> &'static Mutex<Option<SdkTracerProvider>> {
    PROVIDER.get_or_init(|| Mutex::new(None))
}

/// 判断 trace 是否启用
pub fn is_trace_enabled() -> bool {
    TRACE_ENABLED.load(Ordering::SeqCst)
}

/// 获取指定名称的 Tracer
pub fn get_tracer(name: &str) -> opentelemetry::global::BoxedTracer {
    global::tracer(name.to_string())
}

/// 初始化 XTrace
pub fn init_xtrace() -> Result<(), crate::error::XOneError> {
    let config = load_config();

    if !config.is_enabled() {
        xutil::info_if_enable_debug("XTrace disabled by config");
        TRACE_ENABLED.store(false, Ordering::SeqCst);
        return Ok(());
    }

    let service_name = xconfig::get_server_name();

    let provider = if config.console {
        xutil::info_if_enable_debug("XTrace init with console exporter");
        let exporter = opentelemetry_stdout::SpanExporter::default();
        SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_resource(Resource::builder().with_service_name(service_name).build())
            .build()
    } else {
        xutil::info_if_enable_debug("XTrace init with noop (no exporter configured)");
        SdkTracerProvider::builder()
            .with_resource(Resource::builder().with_service_name(service_name).build())
            .build()
    };

    global::set_tracer_provider(provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

    let mut store = provider_store().lock();
    *store = Some(provider);

    TRACE_ENABLED.store(true, Ordering::SeqCst);
    xutil::info_if_enable_debug("XTrace init success");
    Ok(())
}

/// 关闭 XTrace
///
/// 超时由 xhook 框架通过 `HookOptions.timeout` 统一控制。
pub fn shutdown_xtrace() -> Result<(), crate::error::XOneError> {
    if !TRACE_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }

    xutil::info_if_enable_debug("XTrace shutdown begin");

    let provider = {
        let mut store = provider_store().lock();
        store.take()
    };

    if let Some(provider) = provider {
        provider.shutdown().map_err(|e| {
            TRACE_ENABLED.store(false, Ordering::SeqCst);
            crate::error::XOneError::Other(format!("XTrace shutdown failed: {e}"))
        })?;
    }

    TRACE_ENABLED.store(false, Ordering::SeqCst);
    xutil::info_if_enable_debug("XTrace shutdown success");
    Ok(())
}

/// 加载 XTrace 配置
fn load_config() -> XTraceConfig {
    xconfig::parse_config::<XTraceConfig>(XTRACE_CONFIG_KEY).unwrap_or_default()
}
