//! 文件系统相关工具

/// 检查文件是否存在（且不是目录）
///
/// # Examples
///
/// ```
/// // Cargo.toml 在项目根目录下肯定存在
/// assert!(x_one::xutil::file_exist("Cargo.toml"));
/// assert!(!x_one::xutil::file_exist("nonexistent_file.txt"));
/// ```
pub fn file_exist(path: &str) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

/// 检查目录是否存在（且不是文件）
///
/// # Examples
///
/// ```
/// assert!(x_one::xutil::dir_exist("src"));
/// assert!(!x_one::xutil::dir_exist("nonexistent_dir"));
/// ```
pub fn dir_exist(path: &str) -> bool {
    std::fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
}


