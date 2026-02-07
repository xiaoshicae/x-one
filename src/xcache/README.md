# XCache - 本地缓存模块

💡 基于 [moka](https://github.com/moka-rs/moka) 的高性能本地缓存模块，支持 TTL、TinyLFU 淘汰策略、并发安全、泛型 API。

## 🚀 快速开始

### 1. 配置 (application.yml)

```yaml
# 单实例模式
XCache:
  MaxCapacity: 100000    # 最大缓存容量 (默认 100000)
  DefaultTTL: "5m"       # 默认过期时间

# 多实例模式
XCache:
  - Name: "user-cache"
    MaxCapacity: 50000
    DefaultTTL: "10m"
  - Name: "product-cache"
    MaxCapacity: 200000
    DefaultTTL: "1h"
```

> 提示：无需配置也可直接使用，模块会自动懒初始化一个默认缓存实例。

### 2. 使用全局便捷 API (推荐)

最简单的用法，使用默认缓存实例。

```rust
use x_one::xcache;
use std::time::Duration;

// 设置 (支持任意实现了 Clone + Send + Sync + 'static 的类型)
xcache::set("config:rate_limit", 100);

// 设置带 TTL
xcache::set_with_ttl("session:abc", "token_123", Duration::from_secs(3600));

// 获取 (支持类型推断)
// 显式指定类型：
if let Some(limit) = xcache::get::<i32>("config:rate_limit") {
    println!("Rate limit: {}", limit);
}

// 删除
xcache::del("config:rate_limit");
```

### 3. 多实例访问

通过 `xcache::c("name")` 获取指定名称的缓存实例。

```rust
use x_one::xcache;

// 获取名为 "product-cache" 的实例
if let Some(cache) = xcache::c("product-cache") {
    // 使用实例方法
    cache.set("p:1", product);
    
    // 获取
    let p: Option<Product> = cache.get("p:1");
}
```

## ⚙️ 核心特性

- **高性能**：底层基于 `moka`，在高并发场景下表现优异。
- **泛型支持**：`get/set` 方法支持泛型，底层自动处理 `Any` 类型转换。
- **驱逐策略**：使用 TinyLFU 算法，有效提升缓存命中率。
- **自动管理**：集成框架生命周期，自动初始化和优雅关闭。

## ⚠️ 注意事项

1. **类型匹配**：`get` 时指定的类型必须与 `set` 时的类型完全一致，否则会返回 `None`（不会 panic）。
2. **TTL 策略**：目前的 `set_with_ttl` 暂未完全实现 per-entry TTL (依赖 moka 的 policy 配置)，建议在配置文件中统一设置 `DefaultTTL`。
3. **结构体存储**：存入缓存的结构体必须实现 `Clone + Send + Sync + 'static` Trait。