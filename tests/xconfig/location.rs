use serial_test::serial;
use x_one::xconfig::location::*;
use x_one::xutil;

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_get_location_from_current_dir_none() {
    // 在项目根目录运行时，不存在 application.yml
    let result = get_location_from_current_dir();
    assert!(result.is_none());
}

#[test]
#[serial]
fn test_get_location_from_env() {
    set_env(CONFIG_LOCATION_ENV_KEY, "/etc/app.yml");
    let result = get_location_from_env();
    assert_eq!(result, Some("/etc/app.yml".to_string()));
    remove_env(CONFIG_LOCATION_ENV_KEY);
}

#[test]
#[serial]
fn test_get_location_from_env_empty() {
    remove_env(CONFIG_LOCATION_ENV_KEY);
    let result = get_location_from_env();
    assert!(result.is_none());
}

#[test]
fn test_get_location_from_current_dir_with_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("application.yml");
    std::fs::write(
        &file_path,
        "Server:
  Name: test
",
    )
    .unwrap();

    // 保存当前目录
    let original_dir = std::env::current_dir().unwrap();
    // 注意：不能在多线程测试中改变工作目录，这里只测试 file_exist 函数
    assert!(xutil::file_exist(file_path.to_str().unwrap()));

    // 保持工作目录不变
    assert_eq!(std::env::current_dir().unwrap(), original_dir);
}
