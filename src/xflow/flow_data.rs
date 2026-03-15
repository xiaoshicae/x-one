//! 流程数据容器
//!
//! `FlowData<Req, Resp>` 提供 Request/Response/Extra 分离的数据模型，
//! 与 Go 版本的 `FlowData[Req, Resp]` 对齐。

use std::any::Any;
use std::collections::HashMap;

/// 流程数据容器
///
/// 贯穿所有 Processor 的数据容器：
/// - `request`: 入参（语义上不可变）
/// - `response`: 出参（由 Processor 逐步填充）
/// - `extra`: Processor 间临时数据（惰性初始化）
///
/// # Examples
///
/// ```
/// use x_one::xflow::FlowData;
///
/// let mut data = FlowData::new("hello".to_string());
/// assert_eq!(data.request, "hello");
///
/// data.response = 42;
/// assert_eq!(data.response, 42);
///
/// data.set("key", "value".to_string());
/// assert_eq!(data.get::<String>("key"), Some(&"value".to_string()));
/// ```
pub struct FlowData<Req, Resp: Default = ()> {
    /// 入参，语义上不可变
    pub request: Req,
    /// 出参，由 Processor 逐步填充
    pub response: Resp,
    /// Processor 间临时数据（惰性初始化）
    extra: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
}

impl<Req, Resp: Default> FlowData<Req, Resp> {
    /// 创建新的流程数据容器
    pub fn new(request: Req) -> Self {
        Self {
            request,
            response: Resp::default(),
            extra: None,
        }
    }

    /// 创建带初始 response 的容器
    pub fn with_response(request: Req, response: Resp) -> Self {
        Self {
            request,
            response,
            extra: None,
        }
    }

    /// 存储临时数据
    pub fn set<V: Any + Send + Sync>(&mut self, key: impl Into<String>, val: V) {
        self.extra
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), Box::new(val));
    }

    /// 获取临时数据引用
    pub fn get<V: Any + Send + Sync>(&self, key: &str) -> Option<&V> {
        self.extra
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(|v| v.downcast_ref())
    }

    /// 获取临时数据可变引用
    pub fn get_mut<V: Any + Send + Sync>(&mut self, key: &str) -> Option<&mut V> {
        self.extra
            .as_mut()
            .and_then(|m| m.get_mut(key))
            .and_then(|v| v.downcast_mut())
    }

    /// 移除并返回临时数据
    pub fn remove<V: Any + Send + Sync>(&mut self, key: &str) -> Option<V> {
        self.extra
            .as_mut()
            .and_then(|m| m.remove(key))
            .and_then(|v| v.downcast().ok())
            .map(|v| *v)
    }

    /// 检查是否包含指定临时数据
    pub fn contains_key(&self, key: &str) -> bool {
        self.extra.as_ref().is_some_and(|m| m.contains_key(key))
    }
}
