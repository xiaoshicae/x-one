# XOrm - 数据库模块

💡 基于 [sqlx](https://github.com/launchbadge/sqlx) 封装，提供 MySQL/PostgreSQL 连接池管理、多数据库支持、自动配置。

## 功能特性

- **多驱动支持**：支持 `mysql` 和 `postgres`。
- **连接池管理**：可配置最大连接数、空闲连接数、生命周期等。
- **多实例**：支持同时连接多个数据库（如主库、从库）。
- **异步非阻塞**：完全基于 `sqlx` 的异步 API。

## 配置

### 单数据库

```yaml
XOrm:
  Driver: "mysql"          # mysql 或 postgres
  DSN: "user:pass@tcp(127.0.0.1:3306)/dbname"
  MaxOpenConns: 50         # 最大连接数
  MaxIdleConns: 10         # 最小空闲连接数
  MaxLifetime: "1h"        # 连接最长存活时间
  MaxIdleTime: "10m"       # 空闲连接回收时间
```

### 多数据库

```yaml
XOrm:
  - Name: "master"
    Driver: "mysql"
    DSN: "..."
  - Name: "slave"
    Driver: "postgres"
    DSN: "..."
```

## 使用 Demo

```rust
use x_one::xorm;
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取默认连接池
    if let Some(pool) = xorm::get_pool(None) {
        let row: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await?;
        println!("Result: {}", row.0);
    }

    // 获取指定名称的连接池
    if let Some(slave_pool) = xorm::get_pool(Some("slave")) {
        // ...
    }

    Ok(())
}
```

## 注意事项

- `DSN` 格式需符合 `sqlx` 要求（例如 MySQL 推荐使用 `mysql://` 前缀，虽然配置中为了兼容旧版可能支持其他格式，具体请参考 `sqlx` 文档）。
- 模块会在启动时尝试连接数据库，如果连接失败会记录错误但不强制 panic（取决于 `must_invoke_success` 配置，当前默认策略较为宽松）。