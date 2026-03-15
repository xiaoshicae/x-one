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

// ---- Debug impl 与 sanitize_dsn 覆盖 ----

/// 构造一个仅设置 dsn 字段的测试用 XOrmConfig
fn config_with_dsn(dsn: &str) -> XOrmConfig {
    XOrmConfig {
        dsn: dsn.to_string(),
        ..Default::default()
    }
}

#[test]
fn test_debug_masks_password_in_postgres_dsn() {
    let config = config_with_dsn("postgres://user:secret123@localhost:5432/mydb");
    let debug_output = format!("{:?}", config);
    assert!(
        debug_output.contains("user:***@localhost"),
        "密码应被脱敏，实际输出: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("secret123"),
        "原始密码不应出现在 Debug 输出中"
    );
}

#[test]
fn test_debug_masks_password_in_mysql_dsn() {
    let config = config_with_dsn("mysql://admin:p@ssw0rd@db.example.com:3306/app");
    let debug_output = format!("{:?}", config);
    // 注意: 密码中包含 '@'，sanitize_dsn 使用第一个 '@' 作为分隔，
    // 所以 userinfo 是 "admin:p"，密码是 "p"
    assert!(
        !debug_output.contains("p@ssw0rd"),
        "完整密码不应出现在 Debug 输出中"
    );
    assert!(debug_output.contains("***"), "应包含脱敏占位符 '***'");
}

#[test]
fn test_debug_no_password_in_dsn() {
    let config = config_with_dsn("postgres://user@localhost:5432/mydb");
    let debug_output = format!("{:?}", config);
    // userinfo 中无冒号，不做脱敏处理，原样保留
    assert!(
        debug_output.contains("postgres://user@localhost:5432/mydb"),
        "无密码的 DSN 应原样保留，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_no_userinfo_in_dsn() {
    let config = config_with_dsn("postgres://localhost:5432/mydb");
    let debug_output = format!("{:?}", config);
    // 无 '@' 符号，不做脱敏处理
    assert!(
        debug_output.contains("postgres://localhost:5432/mydb"),
        "无 userinfo 的 DSN 应原样保留，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_no_scheme_in_dsn() {
    let config = config_with_dsn("localhost:5432/mydb");
    let debug_output = format!("{:?}", config);
    // 无 "://" 则不做脱敏处理，原样返回
    assert!(
        debug_output.contains("localhost:5432/mydb"),
        "无 scheme 的 DSN 应原样保留，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_empty_dsn() {
    let config = XOrmConfig::default();
    let debug_output = format!("{:?}", config);
    // 空 DSN 应无问题
    assert!(
        debug_output.contains("dsn: \"\""),
        "空 DSN 应显示空字符串，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_contains_all_fields() {
    let config = XOrmConfig {
        dsn: "postgres://u:p@h/d".to_string(),
        name: "test-db".to_string(),
        ..Default::default()
    };
    let debug_output = format!("{:?}", config);
    // 验证所有字段都出现在 Debug 输出中
    assert!(debug_output.contains("driver:"));
    assert!(debug_output.contains("dsn:"));
    assert!(debug_output.contains("max_open_conns:"));
    assert!(debug_output.contains("max_idle_conns:"));
    assert!(debug_output.contains("max_lifetime:"));
    assert!(debug_output.contains("max_idle_time:"));
    assert!(debug_output.contains("slow_threshold:"));
    assert!(debug_output.contains("enable_log:"));
    assert!(debug_output.contains("name:"));
}

#[test]
fn test_debug_masks_password_with_special_characters() {
    let config = config_with_dsn("postgres://user:p%40ss%3Aword@localhost/db");
    let debug_output = format!("{:?}", config);
    assert!(
        debug_output.contains("user:***@localhost"),
        "URL 编码的密码也应被脱敏，实际输出: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("p%40ss%3Aword"),
        "编码后的密码不应出现在输出中"
    );
}

#[test]
fn test_debug_masks_empty_password() {
    let config = config_with_dsn("postgres://user:@localhost/db");
    let debug_output = format!("{:?}", config);
    // 空密码时冒号仍然存在，应被替换为 ***
    assert!(
        debug_output.contains("user:***@localhost"),
        "空密码也应被脱敏，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_masks_long_password() {
    let config =
        config_with_dsn("postgres://admin:super-long-secret-password-123!@db.prod.com:5432/app");
    let debug_output = format!("{:?}", config);
    assert!(
        debug_output.contains("admin:***@db.prod.com"),
        "长密码应被脱敏，实际输出: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("super-long-secret-password-123!"),
        "原始长密码不应出现在输出中"
    );
}

#[test]
fn test_debug_dsn_with_query_params() {
    let config =
        config_with_dsn("postgres://user:pass@localhost:5432/db?sslmode=require&timeout=30");
    let debug_output = format!("{:?}", config);
    assert!(
        debug_output.contains("user:***@localhost:5432/db?sslmode=require&timeout=30"),
        "查询参数应被保留，密码应被脱敏，实际输出: {}",
        debug_output
    );
    assert!(!debug_output.contains(":pass@"), "密码不应出现在输出中");
}

#[test]
fn test_debug_dsn_scheme_only() {
    // 只有 scheme 和 "://"，后面没有有效的 userinfo
    let config = config_with_dsn("postgres://");
    let debug_output = format!("{:?}", config);
    // 无 '@' 符号，不做脱敏处理，原样保留
    assert!(
        debug_output.contains("postgres://"),
        "仅 scheme 的 DSN 应原样保留，实际输出: {}",
        debug_output
    );
}

#[test]
fn test_debug_dsn_with_port_no_user() {
    let config = config_with_dsn("mysql://127.0.0.1:3306/mydb");
    let debug_output = format!("{:?}", config);
    // 无 '@'，不做脱敏
    assert!(
        debug_output.contains("mysql://127.0.0.1:3306/mydb"),
        "无用户信息的 MySQL DSN 应原样保留，实际输出: {}",
        debug_output
    );
}
