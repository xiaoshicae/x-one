//! xmetric 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;

use super::config::{XMETRIC_CONFIG_KEY, XMetricConfig};

/// 初始化 xmetric
pub fn init_xmetric() -> Result<(), crate::error::XOneError> {
    let config = if xconfig::contain_key(XMETRIC_CONFIG_KEY) {
        xconfig::parse_config::<XMetricConfig>(XMETRIC_CONFIG_KEY).unwrap_or_default()
    } else {
        xutil::info_if_enable_debug("XMetric config not found, using defaults");
        XMetricConfig::default()
    };

    xutil::info_if_enable_debug(&format!(
        "XMetric init, namespace=[{}], const_labels=[{:?}]",
        config.namespace, config.const_labels
    ));

    super::client::set_config(config);
    Ok(())
}

/// 关闭 xmetric
pub fn shutdown_xmetric() -> Result<(), crate::error::XOneError> {
    xutil::info_if_enable_debug("XMetric shutdown");
    Ok(())
}
