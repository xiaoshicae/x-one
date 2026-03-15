//! 请求上下文中间件
//!
//! 为每个请求注入 session 上下文（如 trace ID），
//! 确保后续中间件和 handler 可以通过 Extension 获取。

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::collections::HashMap;
use std::sync::Arc;

/// 请求上下文 KV 容器
///
/// 存储在 request extensions 中，供后续 handler 和中间件读取。
#[derive(Clone, Debug, Default)]
pub struct SessionContext {
    /// 键值对存储
    pub kvs: Arc<parking_lot::RwLock<HashMap<String, String>>>,
}

impl SessionContext {
    /// 创建空的 session 上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置键值对
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.kvs.write().insert(key.into(), value.into());
    }

    /// 获取值
    pub fn get(&self, key: &str) -> Option<String> {
        self.kvs.read().get(key).cloned()
    }
}

/// Session 中间件
///
/// 为每个请求创建一个 `SessionContext` 并注入到 request extensions 中。
///
/// # Examples
///
/// ```ignore
/// use axum::{Router, middleware};
/// use x_one::xaxum::middleware::session_middleware;
///
/// let app = Router::new()
///     .layer(middleware::from_fn(session_middleware));
/// ```
pub async fn session_middleware(mut req: Request<Body>, next: Next) -> Response {
    let ctx = SessionContext::new();
    req.extensions_mut().insert(ctx);
    next.run(req).await
}
