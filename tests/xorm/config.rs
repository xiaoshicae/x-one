use x_one::xorm::config::*;

#[test]
fn test_default_config() {
    let config = XOrmConfig::default();
    assert_eq!(config.driver, Driver::Postgres);
    assert!(config.dsn.is_empty());
    assert_eq!(config.max_open_conns, 100);
    assert_eq!(config.max_idle_conns, 10);
    assert_eq!(config.max_lifetime, "1h");
    assert_eq!(config.max_idle_time, "10m");
    assert_eq!(config.slow_threshold, "200ms");
    assert!(config.enable_log);
    assert!(config.name.is_empty());
}

#[test]
fn test_driver_display() {
    assert_eq!(Driver::Postgres.to_string(), "postgres");
    assert_eq!(Driver::Mysql.to_string(), "mysql");
}

#[test]
fn test_driver_default() {
    assert_eq!(Driver::default(), Driver::Postgres);
}

#[test]
fn test_deserialize_postgres_config() {
    let yaml = r#"
Driver: "postgres"
DSN: "postgres://user:pass@localhost:5432/db"
MaxOpenConns: 200
MaxIdleConns: 20
MaxLifetime: "2h"
MaxIdleTime: "30m"
SlowThreshold: "500ms"
EnableLog: false
Name: "primary"
"#;
    let config: XOrmConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.driver, Driver::Postgres);
    assert_eq!(config.dsn, "postgres://user:pass@localhost:5432/db");
    assert_eq!(config.max_open_conns, 200);
    assert_eq!(config.max_idle_conns, 20);
    assert_eq!(config.max_lifetime, "2h");
    assert!(!config.enable_log);
    assert_eq!(config.name, "primary");
}

#[test]
fn test_deserialize_mysql_config() {
    let yaml = r#"
Driver: "mysql"
DSN: "mysql://user:pass@localhost:3306/db"
"#;
    let config: XOrmConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.driver, Driver::Mysql);
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XOrmConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.driver, Driver::Postgres);
    assert_eq!(config.max_open_conns, 100);
    assert!(config.enable_log);
}
