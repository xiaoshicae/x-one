//! 服务启动 Banner
//!
//! 在 Axum 服务器启动时打印 ASCII art 标识。

/// ASCII art Banner
const BANNER_TXT: &str = r#"
 __  __     _     __  __ _   _ __  __
 \ \/ /    / \   \ \/ /| | | |  \/  |
  \  /    / _ \   \  / | | | | |\/| |
  /  \   / ___ \  /  \ | |_| | |  | |
 /_/\_\ /_/   \_\/_/\_\ \___/|_|  |_|
"#;

/// 当前 crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 打印启动 Banner
///
/// 输出 ASCII art + 绿色名称 + 版本号到标准输出。
pub fn print_banner() {
    println!("{BANNER_TXT}");
    println!(" \x1b[32m::     XAXUM     ::\x1b[0m  (v{VERSION})");
    println!();
}
