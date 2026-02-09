# XOrm - 数据库模块

基于 [sqlx](https://github.com/launchbadge/sqlx) 封装，提供 MySQL / PostgreSQL 连接池配置管理、多数据库支持。

## 功能特性

- **多驱动支持**：支持 `mysql` 和 `postgres`
- **连接池配置**：可配置最大连接数、空闲连接数、生命周期等
- **多实例**：支持同时管理多个数据库配置（如主库、从库）

## 配置

### 单数据库

```yaml
XOrm:
  Driver: "mysql"                # mysql 或 postgres
  DSN: "mysql://user:pass@127.0.0.1:3306/dbname"
  MaxOpenConns: 50               # 最大连接数
  MaxIdleConns: 10               # 最小空闲连接数
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

```rust
use x_one::xorm;

// 获取默认连接池配置
if let Some(config) = xorm::get_pool_config(None) {
    println!("Driver: {:?}, DSN: {}", config.driver, config.dsn);
}

// 获取指定名称的配置
if let Some(config) = xorm::get_pool_config(Some("slave")) {
    println!("Slave DSN: {}", config.dsn);
}

// 获取驱动类型
let driver = xorm::get_driver(None);  // Option<Driver>

// 获取 DSN
let dsn = xorm::get_dsn(None);  // Option<String>

// 获取所有配置名称
let names = xorm::get_pool_names();  // Vec<String>
```

## 注意事项

- 模块当前提供连接池配置管理，用户可基于 `get_pool_config` 获取配置后自行创建 sqlx 连接池
- DSN 格式需符合 sqlx 要求（MySQL 使用 `mysql://` 前缀，PostgreSQL 使用 `postgres://` 前缀）
