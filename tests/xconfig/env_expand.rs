use serial_test::serial;
use x_one::xconfig::env_expand::*;

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) };
}

#[test]
#[serial]
fn test_expand_env_placeholder_simple() {
    set_env("TEST_VAR_A", "hello");
    let result = expand_env_placeholder("${TEST_VAR_A}");
    assert_eq!(result, "hello");
    remove_env("TEST_VAR_A");
}

#[test]
#[serial]
fn test_expand_env_placeholder_with_default() {
    remove_env("TEST_VAR_B");
    let result = expand_env_placeholder("${TEST_VAR_B:-default_value}");
    assert_eq!(result, "default_value");
}

#[test]
#[serial]
fn test_expand_env_placeholder_env_overrides_default() {
    set_env("TEST_VAR_C", "real_value");
    let result = expand_env_placeholder("${TEST_VAR_C:-default_value}");
    assert_eq!(result, "real_value");
    remove_env("TEST_VAR_C");
}

#[test]
#[serial]
fn test_expand_env_placeholder_missing_no_default() {
    remove_env("TEST_VAR_D");
    let result = expand_env_placeholder("${TEST_VAR_D}");
    assert_eq!(result, "");
}

#[test]
#[serial]
fn test_expand_env_placeholder_multiple() {
    set_env("TEST_HOST", "localhost");
    set_env("TEST_PORT", "8080");
    let result = expand_env_placeholder("${TEST_HOST}:${TEST_PORT}");
    assert_eq!(result, "localhost:8080");
    remove_env("TEST_HOST");
    remove_env("TEST_PORT");
}

#[test]
#[serial]
fn test_expand_env_placeholder_no_placeholder() {
    let result = expand_env_placeholder("no placeholder here");
    assert_eq!(result, "no placeholder here");
}

#[test]
#[serial]
fn test_expand_env_placeholders_in_value() {
    set_env("TEST_EXPAND_VAL", "expanded");
    let yaml_str = "key: ${TEST_EXPAND_VAL}";
    let mut value: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
    expand_env_placeholders_in_value(&mut value);

    let map = value.as_mapping().unwrap();
    let v = map
        .get(&serde_yaml::Value::String("key".to_string()))
        .unwrap();
    assert_eq!(v.as_str().unwrap(), "expanded");
    remove_env("TEST_EXPAND_VAL");
}
