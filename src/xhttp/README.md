# XHttp - HTTP 客户端模块

💡 基于 [reqwest](https://github.com/seanmonstar/reqwest) 封装，提供配置驱动的 HTTP 客户端，支持连接池管理、重试机制、超时控制。

## 配置参数

```yaml
XHttp:
  Timeout: "60s"             # 整体请求超时 (默认 "30s")
  DialTimeout: "10s"         # 连接超时 (默认 "10s")
  DialKeepAlive: "30s"       # TCP KeepAlive 时间 (默认 "30s")
  PoolMaxIdlePerHost: 10     # 每个 Host 最大空闲连接数 (默认 10)
  RetryCount: 3              # 重试次数 (默认 0，不重试)
  RetryWaitTime: "100ms"     # 重试等待时间 (默认 "1s")
  RetryMaxWaitTime: "2s"     # 最大重试等待时间 (默认 "10s")
```

## 使用 Demo

```rust
use x_one::xhttp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取全局客户端（单例）
    let client = xhttp::client();

    // 发起请求
    let resp = client.get("https://httpbin.org/get")
        .header("User-Agent", "x-one-client")
        .send()
        .await?;

    println!("Status: {}", resp.status());
    println!("Body: {}", resp.text().await?);

    Ok(())
}
```

## 注意事项

- **线程安全**：底层 `reqwest::Client` 是线程安全的，建议全局复用。
- **配置生效**：必须调用 `x_one::init_all()` 或 `xhttp::init()` 后，配置才会生效，否则使用默认配置。