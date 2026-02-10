use x_one::xorm::DbPool;
use x_one::xorm::config::Driver;

/// 辅助函数：创建 lazy Postgres 池用于测试
fn make_pg_pool() -> DbPool {
    let pool = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost:5432/test")
        .unwrap();
    DbPool::Postgres(pool)
}

/// 辅助函数：创建 lazy MySQL 池用于测试
fn make_mysql_pool() -> DbPool {
    let pool = sqlx::pool::PoolOptions::<sqlx::MySql>::new()
        .max_connections(1)
        .connect_lazy("mysql://test:test@localhost:3306/test")
        .unwrap();
    DbPool::MySql(pool)
}

#[tokio::test]
async fn test_db_pool_as_postgres_returns_some() {
    let pool = make_pg_pool();
    assert!(pool.as_postgres().is_some());
}

#[tokio::test]
async fn test_db_pool_as_mysql_returns_some() {
    let pool = make_mysql_pool();
    assert!(pool.as_mysql().is_some());
}

#[tokio::test]
async fn test_db_pool_as_wrong_type_returns_none() {
    let pg = make_pg_pool();
    assert!(pg.as_mysql().is_none());

    let my = make_mysql_pool();
    assert!(my.as_postgres().is_none());
}

#[tokio::test]
async fn test_db_pool_driver_postgres() {
    let pool = make_pg_pool();
    assert_eq!(pool.driver(), Driver::Postgres);
}

#[tokio::test]
async fn test_db_pool_driver_mysql() {
    let pool = make_mysql_pool();
    assert_eq!(pool.driver(), Driver::Mysql);
}

#[tokio::test]
async fn test_db_pool_clone() {
    let pool = make_pg_pool();
    let cloned = pool.clone();
    assert_eq!(cloned.driver(), Driver::Postgres);
}
