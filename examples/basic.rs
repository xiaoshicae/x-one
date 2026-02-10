//! 基础示例：展示 x-one 框架的完整生命周期
//!
//! 运行方式：
//! ```bash
//! cargo run --example basic
//! ```

use axum::Router;
use axum::routing::get;
use x_one::XAxum;

#[tokio::main]
async fn main() {
    // 指定配置文件路径（相对于项目根目录）
    unsafe { std::env::set_var("SERVER_CONFIG_LOCATION", "examples/config/application.yml") };

    // 构建 Axum HTTP 服务器
    let server = XAxum::new().with_route_register(register_routes).build();

    // 运行服务器（自动执行 init + 阻塞等待退出信号 + shutdown）
    if let Err(e) = x_one::run_server(&server).await {
        eprintln!("服务器运行失败: {e}");
    }
}

/// 注册路由
fn register_routes(mut r: Router) -> Router {
    r = r.route("/", get(index));
    r = r.route("/cache", get(cache_demo));
    r
}

/// 首页
async fn index() -> &'static str {
    "hello from x-one"
}

/// 缓存示例：写入并读取缓存
async fn cache_demo() -> String {
    x_one::xcache::set("greeting", "hello from x-one".to_string());
    match x_one::xcache::get::<String>("greeting") {
        Some(msg) => format!("缓存命中: {msg}"),
        None => "缓存未命中".to_string(),
    }
}
