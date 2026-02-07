//! 环境变量相关工具

/// 环境变量 key，用于启用 debug 模式
pub const DEBUG_KEY: &str = "SERVER_ENABLE_DEBUG";

/// 检查是否启用 debug 模式
///
/// 读取环境变量 `SERVER_ENABLE_DEBUG`，支持以下值表示启用：
/// `true`, `1`, `t`, `yes`, `y`, `on`（不区分大小写）
///
/// # Examples
///
/// ```
/// unsafe { std::env::set_var("SERVER_ENABLE_DEBUG", "true"); }
/// assert!(x_one::xutil::enable_debug());
/// unsafe { std::env::remove_var("SERVER_ENABLE_DEBUG"); }
/// ```
pub fn enable_debug() -> bool {
    match std::env::var(DEBUG_KEY) {
        Ok(val) => matches!(
            val.trim().to_lowercase().as_str(),
            "true" | "1" | "t" | "yes" | "y" | "on"
        ),
        Err(_) => false,
    }
}


