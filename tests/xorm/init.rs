use serial_test::serial;
use x_one::xorm::client::*;
use x_one::xorm::init::*;
use x_one::xorm::*;

#[test]
#[serial]
fn test_init_xorm_no_config() {
    reset_pool_configs();
    let result = init_xorm();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_shutdown_xorm() {
    reset_pool_configs();
    let result = shutdown_xorm();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_get_pool_config_none() {
    reset_pool_configs();
    assert!(get_pool_config(None).is_none());
}

#[test]
#[serial]
fn test_get_pool_config_named() {
    reset_pool_configs();
    set_pool_entry(
        "test_db",
        PoolEntry {
            config: XOrmConfig {
                name: "test_db".to_string(),
                dsn: "postgres://localhost/test".to_string(),
                ..XOrmConfig::default()
            },
        },
    );

    let config = get_pool_config(Some("test_db"));
    assert!(config.is_some());
    assert_eq!(config.unwrap().dsn, "postgres://localhost/test");
    reset_pool_configs();
}

#[test]
#[serial]
fn test_get_pool_names() {
    reset_pool_configs();
    set_pool_entry(
        "db1",
        PoolEntry {
            config: XOrmConfig::default(),
        },
    );
    set_pool_entry(
        "db2",
        PoolEntry {
            config: XOrmConfig::default(),
        },
    );

    let names = get_pool_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"db1".to_string()));
    assert!(names.contains(&"db2".to_string()));
    reset_pool_configs();
}

#[test]
#[serial]
fn test_get_driver() {
    reset_pool_configs();
    set_pool_entry(
        DEFAULT_POOL_NAME,
        PoolEntry {
            config: XOrmConfig {
                driver: Driver::Mysql,
                ..XOrmConfig::default()
            },
        },
    );

    let driver = get_driver(None);
    assert_eq!(driver, Some(Driver::Mysql));
    reset_pool_configs();
}

#[test]
#[serial]
fn test_get_dsn() {
    reset_pool_configs();
    set_pool_entry(
        DEFAULT_POOL_NAME,
        PoolEntry {
            config: XOrmConfig {
                dsn: "postgres://localhost/mydb".to_string(),
                ..XOrmConfig::default()
            },
        },
    );

    let dsn = get_dsn(None);
    assert_eq!(dsn, Some("postgres://localhost/mydb".to_string()));
    reset_pool_configs();
}

#[test]
fn test_load_configs_no_config() {
    let configs = load_configs();
    assert!(configs.is_empty());
}
