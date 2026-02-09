use std::time::Duration;
use x_one::xcache::cache::*;

#[test]
fn test_cache_new() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    assert_eq!(cache.entry_count(), 0);
    assert_eq!(cache.default_ttl(), Duration::from_secs(300));
}

#[test]
fn test_cache_set_and_get_string() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("key1", "hello".to_string());
    let value: Option<String> = cache.get("key1");
    assert_eq!(value, Some("hello".to_string()));
}

#[test]
fn test_cache_set_and_get_i64() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("num", 42_i64);
    let value: Option<i64> = cache.get("num");
    assert_eq!(value, Some(42));
}

#[test]
fn test_cache_get_wrong_type() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("key", "hello".to_string());
    // 尝试以 i64 获取 String 类型的值
    let value: Option<i64> = cache.get("key");
    assert!(value.is_none());
}

#[test]
fn test_cache_get_missing_key() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    let value: Option<String> = cache.get("nonexistent");
    assert!(value.is_none());
}

#[test]
fn test_cache_del() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("key", "value".to_string());
    assert!(cache.get::<String>("key").is_some());
    cache.del("key");
    // moka 的 invalidate 可能不会立即生效，但最终一致
    // 等一小段时间
    std::thread::sleep(Duration::from_millis(50));
    assert!(cache.get::<String>("key").is_none());
}

#[test]
fn test_cache_set_with_ttl() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set_with_ttl("key", "value".to_string(), Duration::from_secs(1));
    let value: Option<String> = cache.get("key");
    assert_eq!(value, Some("value".to_string()));
}

#[test]
fn test_cache_set_with_ttl_expires_independently() {
    // 全局 TTL 300s，per-entry TTL 100ms
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("long_lived", "stays".to_string());
    cache.set_with_ttl(
        "short_lived",
        "goes".to_string(),
        Duration::from_millis(100),
    );

    // 两个 key 都存在
    assert!(cache.get::<String>("long_lived").is_some());
    assert!(cache.get::<String>("short_lived").is_some());

    // 等待 per-entry TTL 过期
    std::thread::sleep(Duration::from_millis(200));

    // 短 TTL 条目已过期，长 TTL 条目仍存在
    assert!(cache.get::<String>("long_lived").is_some());
    assert!(
        cache.get::<String>("short_lived").is_none(),
        "per-entry TTL 应使条目过期"
    );
}

#[test]
fn test_cache_overwrite() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    cache.set("key", "v1".to_string());
    cache.set("key", "v2".to_string());
    let value: Option<String> = cache.get("key");
    assert_eq!(value, Some("v2".to_string()));
}

#[test]
fn test_cache_debug() {
    let cache = Cache::new(1000, Duration::from_secs(300));
    let debug = format!("{:?}", cache);
    assert!(debug.contains("Cache"));
}

#[test]
fn test_cache_custom_struct() {
    #[derive(Clone, Debug, PartialEq)]
    struct User {
        name: String,
        age: u32,
    }

    let cache = Cache::new(1000, Duration::from_secs(300));
    let user = User {
        name: "Alice".to_string(),
        age: 30,
    };
    cache.set("user:1", user.clone());
    let result: Option<User> = cache.get("user:1");
    assert_eq!(result, Some(user));
}
