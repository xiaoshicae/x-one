use serial_test::serial;
use x_one::xorm::client::*;
use x_one::xorm::config::*;
use x_one::xorm::init::*;

#[test]
#[serial]
fn test_init_xorm_no_config() {
    reset_pools();
    let result = init_xorm();
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_shutdown_xorm_clears_pools() {
    reset_pools();
    // 手动插入一个 pool
    let pool = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost:5432/test")
        .unwrap();
    set_pool(DEFAULT_POOL_NAME, DbPool::Postgres(pool));
    assert!(db().is_some());

    let result = shutdown_xorm();
    assert!(result.is_ok());
    assert!(db().is_none());
}

#[tokio::test]
async fn test_build_pool_postgres_lazy() {
    let config = XOrmConfig {
        driver: Driver::Postgres,
        dsn: "postgres://user:pass@localhost:5432/testdb".to_string(),
        max_open_conns: 50,
        max_idle_conns: 5,
        ..XOrmConfig::default()
    };
    let pool = build_pool(&config);
    assert!(pool.is_ok());
    let pool = pool.unwrap();
    assert_eq!(pool.driver(), Driver::Postgres);
    assert!(pool.as_postgres().is_some());
}

#[tokio::test]
async fn test_build_pool_mysql_lazy() {
    let config = XOrmConfig {
        driver: Driver::Mysql,
        dsn: "mysql://user:pass@localhost:3306/testdb".to_string(),
        max_open_conns: 50,
        max_idle_conns: 5,
        ..XOrmConfig::default()
    };
    let pool = build_pool(&config);
    assert!(pool.is_ok());
    let pool = pool.unwrap();
    assert_eq!(pool.driver(), Driver::Mysql);
    assert!(pool.as_mysql().is_some());
}

#[tokio::test]
async fn test_build_pool_invalid_dsn_returns_error() {
    let config = XOrmConfig {
        driver: Driver::Postgres,
        dsn: "not_a_valid_dsn".to_string(),
        ..XOrmConfig::default()
    };
    let result = build_pool(&config);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_db_returns_none_when_empty() {
    reset_pools();
    assert!(db().is_none());
}

#[tokio::test]
#[serial]
async fn test_db_with_name_returns_pool() {
    reset_pools();
    let pool = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost:5432/test")
        .unwrap();
    set_pool("analytics", DbPool::Postgres(pool));

    let result = db_with_name("analytics");
    assert!(result.is_some());
    assert_eq!(result.unwrap().driver(), Driver::Postgres);
    reset_pools();
}

#[tokio::test]
#[serial]
async fn test_db_default_pool() {
    reset_pools();
    let pool = sqlx::pool::PoolOptions::<sqlx::MySql>::new()
        .max_connections(1)
        .connect_lazy("mysql://test:test@localhost:3306/test")
        .unwrap();
    set_pool(DEFAULT_POOL_NAME, DbPool::MySql(pool));

    let result = db();
    assert!(result.is_some());
    assert_eq!(result.unwrap().driver(), Driver::Mysql);
    reset_pools();
}

#[tokio::test]
#[serial]
async fn test_get_pool_names_returns_all() {
    reset_pools();
    let pg = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost:5432/test")
        .unwrap();
    let my = sqlx::pool::PoolOptions::<sqlx::MySql>::new()
        .max_connections(1)
        .connect_lazy("mysql://test:test@localhost:3306/test")
        .unwrap();
    set_pool("db1", DbPool::Postgres(pg));
    set_pool("db2", DbPool::MySql(my));

    let names = get_pool_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"db1".to_string()));
    assert!(names.contains(&"db2".to_string()));
    reset_pools();
}

#[test]
fn test_config_default() {
    let config = XOrmConfig::default();
    assert_eq!(config.driver, Driver::Postgres);
    assert!(config.dsn.is_empty());
    assert_eq!(config.max_open_conns, 100);
    assert_eq!(config.max_idle_conns, 10);
}
