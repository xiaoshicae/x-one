//! xpipeline 配置

/// Pipeline 配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// channel 缓冲大小，默认 64
    pub buffer_size: usize,
    /// 是否禁用监控，默认 false
    pub disable_monitor: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64,
            disable_monitor: false,
        }
    }
}
