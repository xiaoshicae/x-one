//! 基础示例：展示 x-one 框架的完整生命周期
//!
//! 运行方式：
//! ```bash
//! cargo run --example basic
//! ```

#[tokio::main]
async fn main() {
    // 启动框架（注册 hook 并执行 before_start hooks）
    // 注册顺序：xconfig → xlog → xtrace → xhttp → xorm → xcache
    x_one::start().expect("启动失败");

    // 使用 xcache 进行缓存操作
    x_one::xcache::set("greeting", "hello from x-one".to_string());
    if let Some(msg) = x_one::xcache::get::<String>("greeting") {
        println!("缓存命中: {msg}");
    }

    // 创建 Axum 路由
    let app = axum::Router::new().route("/", axum::routing::get(|| async { "hello from x-one" }));

    // 以 Axum HTTP 服务器运行，阻塞等待退出信号
    // 退出时自动调用 BeforeStop hooks（xtrace shutdown、xorm close、xcache clear 等）
    if let Err(e) = x_one::run_axum(app).await {
        eprintln!("服务器运行失败: {e}");
    }
}
