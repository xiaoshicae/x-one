//! xmetric 对外 API
//!
//! 提供 Counter、Gauge、Histogram 便捷操作。

use parking_lot::RwLock;
use prometheus_client::encoding::{EncodeLabelSet, LabelSetEncoder};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;

use super::config::XMetricConfig;

/// 全局 Registry
static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();

/// 全局配置
static CONFIG: OnceLock<RwLock<XMetricConfig>> = OnceLock::new();

/// Counter 缓存
static COUNTERS: OnceLock<RwLock<HashMap<String, Family<DynLabels, Counter>>>> = OnceLock::new();

/// f64 精度的 Gauge 类型
type F64Gauge = Gauge<f64, AtomicU64>;

/// Gauge 缓存（使用 f64 精度）
static GAUGES: OnceLock<RwLock<HashMap<String, Family<DynLabels, F64Gauge>>>> = OnceLock::new();

/// Histogram 缓存
static HISTOGRAMS: OnceLock<RwLock<HashMap<String, Family<DynLabels, Histogram>>>> =
    OnceLock::new();

/// 动态标签类型
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DynLabels(Vec<(String, String)>);

impl EncodeLabelSet for DynLabels {
    fn encode(&self, mut encoder: LabelSetEncoder<'_>) -> Result<(), std::fmt::Error> {
        use prometheus_client::encoding::EncodeLabel;
        for (k, v) in &self.0 {
            let pair = (k.as_str(), v.as_str());
            pair.encode(encoder.encode_label())?;
        }
        Ok(())
    }
}

fn registry_store() -> &'static RwLock<Registry> {
    REGISTRY.get_or_init(|| RwLock::new(Registry::default()))
}

fn config_store() -> &'static RwLock<XMetricConfig> {
    CONFIG.get_or_init(|| RwLock::new(XMetricConfig::default()))
}

fn counter_store() -> &'static RwLock<HashMap<String, Family<DynLabels, Counter>>> {
    COUNTERS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn gauge_store() -> &'static RwLock<HashMap<String, Family<DynLabels, F64Gauge>>> {
    GAUGES.get_or_init(|| RwLock::new(HashMap::new()))
}

fn histogram_store() -> &'static RwLock<HashMap<String, Family<DynLabels, Histogram>>> {
    HISTOGRAMS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 获取全局 Registry 引用（用于暴露 /metrics 端点）
pub fn registry() -> &'static RwLock<Registry> {
    registry_store()
}

/// 设置全局配置（由 init 调用）
pub(crate) fn set_config(config: XMetricConfig) {
    *config_store().write() = config;
}

/// 获取全局配置
pub(crate) fn get_config() -> XMetricConfig {
    config_store().read().clone()
}

/// 递增计数器 +1
///
/// # Examples
/// ```ignore
/// x_one::xmetric::counter_inc("http_requests_total", &[("method", "GET")]);
/// ```
pub fn counter_inc(name: &str, labels: &[(&str, &str)]) {
    counter_add(name, 1, labels);
}

/// 递增计数器 +v
pub fn counter_add(name: &str, v: u64, labels: &[(&str, &str)]) {
    let dyn_labels = to_dyn_labels(labels);
    let family = get_or_create_counter(name);
    family.get_or_create(&dyn_labels).inc_by(v);
}

/// 设置仪表盘值
pub fn gauge_set(name: &str, v: f64, labels: &[(&str, &str)]) {
    let dyn_labels = to_dyn_labels(labels);
    let family = get_or_create_gauge(name);
    family.get_or_create(&dyn_labels).set(v);
}

/// 仪表盘 +1
pub fn gauge_inc(name: &str, labels: &[(&str, &str)]) {
    let dyn_labels = to_dyn_labels(labels);
    let family = get_or_create_gauge(name);
    family.get_or_create(&dyn_labels).inc();
}

/// 仪表盘 -1
pub fn gauge_dec(name: &str, labels: &[(&str, &str)]) {
    let dyn_labels = to_dyn_labels(labels);
    let family = get_or_create_gauge(name);
    family.get_or_create(&dyn_labels).dec();
}

/// 直方图观测
pub fn histogram_observe(name: &str, v: f64, labels: &[(&str, &str)]) {
    let dyn_labels = to_dyn_labels(labels);
    let family = get_or_create_histogram(name);
    family.get_or_create(&dyn_labels).observe(v);
}

/// 重置所有指标（仅测试用）
#[doc(hidden)]
pub fn reset_metrics() {
    counter_store().write().clear();
    gauge_store().write().clear();
    histogram_store().write().clear();
    *registry_store().write() = Registry::default();
    *config_store().write() = XMetricConfig::default();
}

fn to_dyn_labels(labels: &[(&str, &str)]) -> DynLabels {
    let mut pairs: Vec<(String, String)> = labels
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    DynLabels(pairs)
}

// 注意：以下三个 get_or_create 函数使用 double-checked locking 模式。
// 写锁路径中先检查 store 是否已存在（防止 TOCTOU 竞态），
// 再在同一写锁保护下注册到 registry，保证不会重复注册。

fn get_or_create_counter(name: &str) -> Family<DynLabels, Counter> {
    {
        let store = counter_store().read();
        if let Some(family) = store.get(name) {
            return family.clone();
        }
    }

    let mut store = counter_store().write();
    if let Some(family) = store.get(name) {
        return family.clone();
    }

    let family = Family::<DynLabels, Counter>::default();
    let metric_name = build_metric_name(name);
    store.insert(name.to_string(), family.clone());
    registry_store()
        .write()
        .register(&metric_name, name, family.clone());
    family
}

fn get_or_create_gauge(name: &str) -> Family<DynLabels, F64Gauge> {
    {
        let store = gauge_store().read();
        if let Some(family) = store.get(name) {
            return family.clone();
        }
    }

    let mut store = gauge_store().write();
    if let Some(family) = store.get(name) {
        return family.clone();
    }

    let family = Family::<DynLabels, F64Gauge>::default();
    let metric_name = build_metric_name(name);
    store.insert(name.to_string(), family.clone());
    registry_store()
        .write()
        .register(&metric_name, name, family.clone());
    family
}

fn get_or_create_histogram(name: &str) -> Family<DynLabels, Histogram> {
    {
        let store = histogram_store().read();
        if let Some(family) = store.get(name) {
            return family.clone();
        }
    }

    let mut store = histogram_store().write();
    if let Some(family) = store.get(name) {
        return family.clone();
    }

    let family = Family::<DynLabels, Histogram>::new_with_constructor(default_histogram);
    let metric_name = build_metric_name(name);
    store.insert(name.to_string(), family.clone());
    registry_store()
        .write()
        .register(&metric_name, name, family.clone());
    family
}

/// 默认直方图构造函数（使用 prometheus 默认桶边界）
fn default_histogram() -> Histogram {
    Histogram::new([
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ])
}

fn build_metric_name(name: &str) -> String {
    let config = get_config();
    if config.namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}_{}", config.namespace, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_config_and_get_config_roundtrip() {
        reset_metrics();

        let mut config = XMetricConfig::default();
        config.namespace = "myapp".to_string();
        set_config(config.clone());

        let got = get_config();
        assert_eq!(
            got.namespace, "myapp",
            "set_config 后 get_config 应返回设置的值"
        );
    }

    #[test]
    #[serial]
    fn test_build_metric_name_without_namespace_returns_raw_name() {
        reset_metrics();

        let name = build_metric_name("http_requests");
        assert_eq!(name, "http_requests");
    }

    #[test]
    #[serial]
    fn test_build_metric_name_with_namespace_returns_prefixed_name() {
        reset_metrics();

        let mut config = XMetricConfig::default();
        config.namespace = "myapp".to_string();
        set_config(config);

        let name = build_metric_name("http_requests");
        assert_eq!(name, "myapp_http_requests");
    }

    #[test]
    #[serial]
    fn test_to_dyn_labels_sorts_by_key() {
        let labels = to_dyn_labels(&[("z_key", "val1"), ("a_key", "val2"), ("m_key", "val3")]);
        let expected = DynLabels(vec![
            ("a_key".to_string(), "val2".to_string()),
            ("m_key".to_string(), "val3".to_string()),
            ("z_key".to_string(), "val1".to_string()),
        ]);
        assert_eq!(labels, expected, "标签应按 key 排序");
    }

    #[test]
    #[serial]
    fn test_dyn_labels_encode_label_set() {
        reset_metrics();

        // 通过 counter_inc 间接触发 DynLabels encode，然后验证 registry 输出
        counter_inc("encode_test", &[("method", "GET")]);
        let mut output = String::new();
        prometheus_client::encoding::text::encode(&mut output, &registry_store().read())
            .expect("encode 不应失败");
        assert!(
            output.contains("method"),
            "编码输出应包含标签 key: {output}"
        );
    }

    #[test]
    #[serial]
    fn test_get_or_create_counter_double_checked_locking() {
        reset_metrics();

        // 第一次调用：走写锁路径创建
        let family1 = get_or_create_counter("dc_counter");
        // 第二次调用：走读锁路径返回缓存
        let family2 = get_or_create_counter("dc_counter");

        // 两次返回的 family 应指向同一数据
        let labels = to_dyn_labels(&[("k", "v")]);
        family1.get_or_create(&labels).inc();
        let val = family2.get_or_create(&labels).get();
        assert_eq!(val, 1, "两次 get_or_create 应返回同一 family");
    }

    #[test]
    #[serial]
    fn test_get_or_create_gauge_double_checked_locking() {
        reset_metrics();

        let family1 = get_or_create_gauge("dc_gauge");
        let family2 = get_or_create_gauge("dc_gauge");

        let labels = to_dyn_labels(&[("k", "v")]);
        family1.get_or_create(&labels).set(42.0);
        let val = family2.get_or_create(&labels).get();
        assert!(
            (val - 42.0).abs() < f64::EPSILON,
            "两次 get_or_create 应返回同一 family"
        );
    }

    #[test]
    #[serial]
    fn test_get_or_create_histogram_double_checked_locking() {
        reset_metrics();

        let family1 = get_or_create_histogram("dc_histogram");
        let family2 = get_or_create_histogram("dc_histogram");

        // 两次返回的 family 应该是同一个实例，observe 不应 panic
        let labels = to_dyn_labels(&[("k", "v")]);
        family1.get_or_create(&labels).observe(1.0);
        family2.get_or_create(&labels).observe(2.0);
    }
}
