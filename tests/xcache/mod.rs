use serial_test::serial;
use std::time::Duration;
use x_one::xcache::*;

fn reset_and_init() {
    let mut store = client::cache_store().write();
    store.clear();
    drop(store);
    init::init_xcache().unwrap();
}

fn cleanup() {
    let mut store = client::cache_store().write();
    store.clear();
}

#[test]
fn test_register_hook_idempotent() {
    register_hook();
    register_hook();
}

#[test]
#[serial]
fn test_convenience_get_set() {
    reset_and_init();
    set("greeting", "hello".to_string());
    let value: Option<String> = get("greeting");
    assert_eq!(value, Some("hello".to_string()));
    cleanup();
}

#[test]
#[serial]
fn test_convenience_set_with_ttl() {
    reset_and_init();
    set_with_ttl("temp", 42_i64, Duration::from_secs(60));
    let value: Option<i64> = get("temp");
    assert_eq!(value, Some(42));
    cleanup();
}

#[test]
#[serial]
fn test_convenience_del() {
    reset_and_init();
    set("to_delete", "bye".to_string());
    del("to_delete");
    std::thread::sleep(Duration::from_millis(50));
    let value: Option<String> = get("to_delete");
    assert!(value.is_none());
    cleanup();
}

#[test]
#[serial]
fn test_get_no_cache_returns_none() {
    cleanup();
    let value: Option<String> = get("anything");
    assert!(value.is_none());
}
