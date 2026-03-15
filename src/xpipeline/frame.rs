//! Pipeline 中传递的数据单元

use std::any::Any;

/// Pipeline 中传递的数据单元 trait
pub trait Frame: Send + 'static {
    /// 返回帧类型标识
    fn frame_type(&self) -> &str;
}

/// 启动信号帧，携带任意上下文
pub struct StartFrame {
    /// 启动上下文
    pub context: Box<dyn Any + Send>,
}

impl StartFrame {
    /// 创建启动帧
    pub fn new(context: impl Any + Send) -> Self {
        Self {
            context: Box::new(context),
        }
    }
}

impl Frame for StartFrame {
    fn frame_type(&self) -> &str {
        "start"
    }
}

/// 结束信号帧
pub struct EndFrame;

impl Frame for EndFrame {
    fn frame_type(&self) -> &str {
        "end"
    }
}

/// 错误信号帧
pub struct ErrorFrame {
    /// 错误
    pub err: Box<dyn std::error::Error + Send>,
    /// 错误消息
    pub message: String,
}

impl Frame for ErrorFrame {
    fn frame_type(&self) -> &str {
        "error"
    }
}

/// 元数据传递帧
pub struct MetadataFrame {
    /// 元数据 key
    pub key: String,
    /// 元数据 value
    pub value: Box<dyn Any + Send>,
}

impl MetadataFrame {
    /// 创建元数据帧
    pub fn new(key: impl Into<String>, value: impl Any + Send) -> Self {
        Self {
            key: key.into(),
            value: Box::new(value),
        }
    }
}

impl Frame for MetadataFrame {
    fn frame_type(&self) -> &str {
        "metadata"
    }
}
