# XHttp - HTTP 客户端模块

基于 [reqwest](https://github.com/seanmonstar/reqwest) 封装，提供配置驱动的 HTTP 客户端，支持连接池管理、重试机制、超时控制。

## 配置参数

```yaml
XHttp:
  Timeout: "60s"             # 整体请求超时（默认 30s）
  DialTimeout: "10s"         # 连接超时（默认 10s）
  DialKeepAlive: "30s"       # TCP KeepAlive 时间（默认 30s）
  PoolMaxIdlePerHost: 10     # 每个 Host 最大空闲连接数（默认 10）
  RetryCount: 3              # 重试次数（默认 0，不重试）
  RetryWaitTime: "100ms"     # 重试等待时间（默认 1s）
  RetryMaxWaitTime: "2s"     # 最大重试等待时间（默认 10s）
```

## 使用

### 获取全局客户端

```rust
use x_one::xhttp;

// 获取全局 reqwest::Client（单例）
let client = xhttp::c();
let resp = client.get("https://httpbin.org/get")
    .header("User-Agent", "x-one")
    .send()
    .await?;
```

### 便捷请求方法

模块提供 `get` / `post` / `put` / `patch` / `delete` / `head` 便捷方法，直接返回 `RequestBuilder`：

```rust
use x_one::xhttp;

// GET
let resp = xhttp::get("https://httpbin.org/get").send().await?;

// POST JSON
let resp = xhttp::post("https://httpbin.org/post")
    .json(&serde_json::json!({"key": "value"}))
    .send()
    .await?;

// PUT
let resp = xhttp::put("https://httpbin.org/put")
    .body("data")
    .send()
    .await?;
```

## 注意事项

- **线程安全**：底层 `reqwest::Client` 是线程安全的，全局复用同一实例
- **配置生效**：框架自动初始化，配置在 `x_one::init()` / `run_axum()` 后生效；未配置时使用 reqwest 默认值
