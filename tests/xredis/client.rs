use serial_test::serial;
use x_one::xredis::{
    DEFAULT_CLIENT_NAME, get_client_names, redis_client, redis_client_with_name, reset_clients,
};

#[test]
#[serial]
fn test_default_client_name() {
    assert_eq!(DEFAULT_CLIENT_NAME, "_default_");
}

#[test]
#[serial]
fn test_redis_client_none_when_empty() {
    reset_clients();
    assert!(redis_client().is_none());
}

#[test]
#[serial]
fn test_redis_client_with_name_none_when_empty() {
    reset_clients();
    assert!(redis_client_with_name("nonexistent").is_none());
}

#[test]
#[serial]
fn test_get_client_names_empty() {
    reset_clients();
    assert!(get_client_names().is_empty());
}
