//! xmetric 配置结构体

use serde::Deserialize;
use std::collections::HashMap;

/// XMetric 配置 key
pub const XMETRIC_CONFIG_KEY: &str = "XMetric";

/// XMetric 配置
#[derive(Debug, Deserialize, Clone)]
pub struct XMetricConfig {
    /// 指标命名空间前缀
    #[serde(rename = "Namespace", default)]
    pub namespace: String,

    /// 全局常量标签
    #[serde(rename = "ConstLabels", default)]
    pub const_labels: HashMap<String, String>,

    /// HTTP 请求耗时直方图桶边界（毫秒）
    #[serde(
        rename = "HttpDurationBuckets",
        default = "default_http_duration_buckets"
    )]
    pub http_duration_buckets: Vec<f64>,

    /// 业务 Histogram 默认桶边界（秒）
    #[serde(rename = "HistogramBuckets", default = "default_histogram_buckets")]
    pub histogram_buckets: Vec<f64>,
}

fn default_http_duration_buckets() -> Vec<f64> {
    vec![
        1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
    ]
}

fn default_histogram_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ]
}

impl Default for XMetricConfig {
    fn default() -> Self {
        Self {
            namespace: String::new(),
            const_labels: HashMap::new(),
            http_duration_buckets: default_http_duration_buckets(),
            histogram_buckets: default_histogram_buckets(),
        }
    }
}
