//! XAxum Builder
//!
//! 提供链式 API 构建 Axum HTTP 服务器。

use super::middleware;
use super::server::XAxumServer;
use crate::xutil;
use axum::Router;
use std::net::SocketAddr;

/// Axum HTTP 服务器构建器
///
/// 通过链式调用配置服务器参数，最终调用 `build()` 构建 `XAxumServer`。
///
/// # Examples
///
/// ```
/// use x_one::xaxum::builder::XAxum;
/// use axum::routing::get;
///
/// let server = XAxum::new()
///     .with_route_register(|r| r.route("/health", get(|| async { "ok" })))
///     .build();
/// ```
pub struct XAxum {
    addr: Option<SocketAddr>,
    enable_banner: Option<bool>,
    use_http2: Option<bool>,
    enable_log_middleware: bool,
    enable_trace_middleware: bool,
    route_registers: Vec<Box<dyn FnOnce(Router) -> Router + Send>>,
    middlewares: Vec<Box<dyn FnOnce(Router) -> Router + Send>>,
    router: Option<Router>,
}

impl XAxum {
    /// 创建默认构建器
    ///
    /// 默认启用日志和追踪中间件，地址从配置或默认值获取。
    pub fn new() -> Self {
        Self {
            addr: None,
            enable_banner: None,
            use_http2: None,
            enable_log_middleware: true,
            enable_trace_middleware: true,
            route_registers: Vec::new(),
            middlewares: Vec::new(),
            router: None,
        }
    }

    /// 从已有 Router 创建构建器
    ///
    /// 跳过 `with_route_register`，直接使用传入的 Router。
    pub fn from_router(router: Router) -> Self {
        Self {
            addr: None,
            enable_banner: None,
            use_http2: None,
            enable_log_middleware: true,
            enable_trace_middleware: true,
            route_registers: Vec::new(),
            middlewares: Vec::new(),
            router: Some(router),
        }
    }

    /// 设置监听地址
    ///
    /// 接受 `"host:port"` 格式字符串，解析失败回退到 `0.0.0.0:8000`。
    pub fn addr(mut self, addr: &str) -> Self {
        self.addr = Some(addr.parse().unwrap_or_else(|e| {
            xutil::warn_if_enable_debug(&format!(
                "parse addr [{addr}] failed, fallback to 0.0.0.0:8000, err=[{e}]"
            ));
            SocketAddr::from(([0, 0, 0, 0], 8000))
        }));
        self
    }

    /// 设置是否打印启动 Banner
    ///
    /// 优先级：builder 手动设置 > 配置文件 `EnableBanner` > 默认 `true`。
    pub fn enable_banner(mut self, enable: bool) -> Self {
        self.enable_banner = Some(enable);
        self
    }

    /// 设置是否启用 h2c（HTTP/2 明文）
    ///
    /// 启用后服务器同时支持 HTTP/1.1 和 HTTP/2 明文连接（h2c 自动检测）。
    /// 优先级：builder 手动设置 > 配置文件 `UseHttp2` > 默认 `false`。
    pub fn use_http2(mut self, enable: bool) -> Self {
        self.use_http2 = Some(enable);
        self
    }

    /// 设置是否启用日志中间件（默认 true）
    pub fn enable_log_middleware(mut self, enable: bool) -> Self {
        self.enable_log_middleware = enable;
        self
    }

    /// 设置是否启用追踪中间件（默认 true）
    pub fn enable_trace_middleware(mut self, enable: bool) -> Self {
        self.enable_trace_middleware = enable;
        self
    }

    /// 注册路由回调（可多次调用）
    ///
    /// 回调按注册顺序依次执行。
    pub fn with_route_register<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Router) -> Router + Send + 'static,
    {
        self.route_registers.push(Box::new(f));
        self
    }

    /// 注入自定义中间件（可多次调用）
    ///
    /// 先注册的中间件更接近 handler（内层）。
    pub fn with_middleware<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Router) -> Router + Send + 'static,
    {
        self.middlewares.push(Box::new(f));
        self
    }

    /// 构建 `XAxumServer`
    ///
    /// 构建顺序：
    /// 1. 创建/使用 Router
    /// 2. 执行 route_registers
    /// 3. 注册用户 middlewares（内层，靠近 handler）
    /// 4. 注册内置中间件（外层，优先级高于用户中间件）
    ///
    /// 配置项（addr、banner、http2）延迟到 `run()` 阶段解析，
    /// 确保 `run_server()` 中的 `init()` 先加载配置。
    ///
    /// 请求处理顺序：trace → log → 用户中间件 → handler
    pub fn build(self) -> XAxumServer {
        // 1. 创建或使用已有 Router
        let mut router = self.router.unwrap_or_default();

        // 2. 执行路由注册回调
        for register in self.route_registers {
            router = register(router);
        }

        // 3. 注册用户自定义中间件
        for mw in self.middlewares {
            router = mw(router);
        }

        // 4. 注册内置中间件（外层，优先级高于用户中间件）
        //    log 先注册（内层），trace 后注册（外层）
        //    请求顺序：trace → log → 用户中间件 → handler
        //    trace 先建立上下文，log 才能打印 trace_id
        if self.enable_log_middleware {
            router = router.layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
                middleware::log_middleware,
            ));
        }

        if self.enable_trace_middleware {
            router = router.layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
                middleware::trace_middleware,
            ));
        }

        // 配置项延迟解析：只传递用户显式设置的 Option 值
        XAxumServer::new(router, self.addr, self.enable_banner, self.use_http2)
    }
}

impl Default for XAxum {
    fn default() -> Self {
        Self::new()
    }
}
