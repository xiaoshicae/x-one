use serial_test::serial;
use x_one::xcache::init::*;
use x_one::xcache::*;

#[test]
#[serial]
fn test_init_xcache_default() {
    reset_cache_store();
    let result = init_xcache();
    assert!(result.is_ok());

    let names = get_cache_names();
    assert!(names.contains(&DEFAULT_CACHE_NAME.to_string()));
    reset_cache_store();
}

#[test]
#[serial]
fn test_shutdown_xcache() {
    reset_cache_store();
    init_xcache().unwrap();
    let result = shutdown_xcache();
    assert!(result.is_ok());
    assert!(get_cache_names().is_empty());
}

#[test]
#[serial]
fn test_cache_get_default() {
    reset_cache_store();
    init_xcache().unwrap();
    let c = default();
    assert!(c.is_some());
    reset_cache_store();
}

#[test]
#[serial]
fn test_cache_get_named_missing() {
    reset_cache_store();
    let c = c("nonexistent");
    assert!(c.is_none());
}

#[test]
#[serial]
fn test_create_cache_instance() {
    reset_cache_store();
    let config = XCacheConfig {
        name: "test_cache".to_string(),
        ..XCacheConfig::default()
    };
    let result = create_cache_instance(&config);
    assert!(result.is_ok());
    assert!(get_cache_names().contains(&"test_cache".to_string()));
    reset_cache_store();
}

#[test]
fn test_load_configs_no_config() {
    let configs = load_configs();
    assert!(configs.is_empty());
}

#[test]
#[serial]
fn test_cache_set_and_get_via_instance() {
    reset_cache_store();
    init_xcache().unwrap();

    let c = default().unwrap();
    c.set("test_key", "test_value".to_string());
    let value: Option<String> = c.get("test_key");
    assert_eq!(value, Some("test_value".to_string()));
    reset_cache_store();
}
