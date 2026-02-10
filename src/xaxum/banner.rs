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
/// 输出 ASCII art（雾蓝→淡紫渐变）+ 名称 + 版本号到标准输出。
pub fn print_banner() {
    // 柔和渐变：雾蓝 → 灰蓝 → 淡紫
    const GRADIENT: [[u8; 3]; 5] = [
        [110, 180, 210],
        [116, 168, 208],
        [124, 152, 204],
        [134, 140, 198],
        [142, 132, 194],
    ];

    for (i, line) in BANNER_TXT.trim().lines().enumerate() {
        let ci = i.min(GRADIENT.len() - 1);
        let [r, g, b] = GRADIENT[ci];
        println!("\x1b[38;2;{r};{g};{b}m{line}\x1b[0m");
    }

    // 信息行
    println!(
        " \x1b[38;2;110;180;210m::\x1b[0m     \x1b[38;2;90;190;160mXAXUM\x1b[0m     \x1b[38;2;110;180;210m::\x1b[0m  \x1b[2m(v{VERSION})\x1b[0m"
    );
    println!();
}
