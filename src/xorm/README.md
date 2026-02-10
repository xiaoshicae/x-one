# XOrm - 数据库模块

基于 [sqlx](https://github.com/launchbadge/sqlx) 封装，提供 MySQL / PostgreSQL 类型化连接池管理，支持多数据库实例。

## 功能特性

- **多驱动支持**：支持 `mysql` 和 `postgres` 类型化连接池
- **连接池配置**：可配置最大连接数、空闲连接数、生命周期等
- **多实例**：支持同时管理多个数据库连接池（如主库、从库）
- **Lazy 初始化**：使用 `connect_lazy` 同步创建，首次使用时才建立连接

## 配置

### 单数据库

```yaml
XOrm:
  Driver: "mysql"                # mysql 或 postgres
  DSN: "mysql://user:pass@127.0.0.1:3306/dbname"
  MaxOpenConns: 50               # 最大连接数（默认 100）
  MaxIdleConns: 10               # 最小空闲连接数（默认 10）
  MaxLifetime: "1h"              # 连接最长存活时间
  MaxIdleTime: "10m"             # 空闲连接回收时间
```

### 多数据库

```yaml
XOrm:
  - Name: "master"
    Driver: "mysql"
    DSN: "mysql://..."
  - Name: "slave"
    Driver: "postgres"
    DSN: "postgres://..."
```

## 使用

### 获取默认连接池

```rust
use x_one::xorm;

// 获取默认连接池
if let Some(pool) = xorm::db() {
    let pg = pool.as_postgres().expect("not postgres");
    // sqlx::query("SELECT 1").execute(pg).await?;
}
```

### 获取命名连接池

```rust
use x_one::xorm;

if let Some(pool) = xorm::db_with_name("slave") {
    let my = pool.as_mysql().expect("not mysql");
    // sqlx::query("SELECT 1").execute(my).await?;
}
```

### 查询驱动类型

```rust
use x_one::xorm;

if let Some(pool) = xorm::db() {
    println!("Driver: {}", pool.driver());
}

// 获取所有连接池名称
let names = xorm::get_pool_names();
```

## DbPool 枚举

`DbPool` 是类型化连接池枚举，Pool 内部基于 Arc，Clone 开销极小：

| 方法 | 返回 | 说明 |
|---|---|---|
| `as_postgres()` | `Option<&Pool<Postgres>>` | 获取 PostgreSQL 池引用 |
| `as_mysql()` | `Option<&Pool<MySql>>` | 获取 MySQL 池引用 |
| `driver()` | `Driver` | 获取驱动类型 |

## 注意事项

- 连接池使用 `connect_lazy` 创建，只解析 URL 不建立连接，首次查询时才真正连接数据库
- DSN 格式需符合 sqlx 要求（MySQL 使用 `mysql://` 前缀，PostgreSQL 使用 `postgres://` 前缀）
- DSN 为空的配置会被自动跳过
