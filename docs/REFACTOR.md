# Rust 风格重构：消除 Go 直译痕迹

> Phase 1+2 功能迁移已完成（217 测试），现将 Go 直译代码重构为地道 Rust 风格。

---

## 批次 A: 清理依赖 + 移除 async_trait

### A1: 删除未使用的依赖
- `Cargo.toml` 中删除 `tower`、`tower-http`、`anyhow`、`async-trait`

### A2: 移除 `#[async_trait::async_trait]` 注解
Rust 2024 edition 原生支持 async fn in trait，不再需要 `async_trait` 宏。

| 文件 | 位置 | 说明 |
|------|------|------|
| `src/xserver/mod.rs` | trait Server 定义 | 删除 trait 上的注解 |
| `src/xserver/mod.rs` | tests 中 MockServer impl | 删除 impl 上的注解 |
| `src/xserver/axum.rs` | AxumServer impl | 删除 impl 上的注解 |
| `src/xserver/axum.rs` | AxumTlsServer impl | 删除 impl 上的注解 |
| `src/xserver/blocking.rs` | BlockingServer impl | 删除 impl 上的注解 |

---

## 批次 B: parking_lot 替代 std locks

### 动机
`std::sync::Mutex/RwLock` 的 `lock()/read()/write()` 返回 `Result`，需要 `.unwrap()`。
`parking_lot` 版本不会 poison，直接返回 guard，消除所有 `.unwrap()` 调用。

### 变更
- `Cargo.toml`: 添加 `parking_lot = "0.12"`
- 保留 `std::sync::OnceLock`（parking_lot 无替代）

| 文件 | 替换 | 消除 unwrap 约 N 处 |
|------|------|---------------------|
| `src/xhook/mod.rs` | `std::sync::Mutex` → `parking_lot::Mutex` | 7+ |
| `src/xconfig/mod.rs` | `std::sync::RwLock` → `parking_lot::RwLock` | 6 |
| `src/xtrace/init.rs` | `std::sync::Mutex` → `parking_lot::Mutex` | 3 |
| `src/xtrace/mod.rs` | `std::sync::Mutex` → `parking_lot::Mutex` | 3 |
| `src/xorm/init.rs` | `std::sync::RwLock` → `parking_lot::RwLock` | 6+ |
| `src/xcache/init.rs` | `std::sync::RwLock` → `parking_lot::RwLock` | 6+ |
| `src/xcache/mod.rs` | 测试中 `.write().unwrap()` | 2 |

### 规则
- `.lock().unwrap()` → `.lock()`
- `.read().unwrap()` → `.read()`
- `.write().unwrap()` → `.write()`

---

## 批次 C: retry 泛型化

### 变更
```rust
// Before
pub fn retry<F>(f: F, attempts: usize, sleep: Duration) -> Result<(), String>
where F: Fn() -> Result<(), String>

// After
pub fn retry<F, E>(f: F, attempts: usize, sleep: Duration) -> Result<(), E>
where F: Fn() -> Result<(), E>
```

- `retry_async` 同理加 `E` 泛型参数
- `unwrap_or_else(|| "unknown error")` → `expect("at least one attempt")`
- doc test 加类型标注: `retry(|| Ok::<(), String>(()), 3, ...)`

---

## 批次 D: to_duration 返回 Result

### 变更
```rust
// Before
pub fn to_duration(s: &str) -> Duration

// After
pub fn to_duration(s: &str) -> Result<Duration, String>
```

- `parse_go_duration` 同步改为 `Result<Duration, String>`
- 内部 `error_if_enable_debug(...); return Duration::ZERO` → `return Err("...")`

### 调用方适配

| 文件 | 改法 |
|------|------|
| `src/xhttp/init.rs` (3 处) | `.unwrap_or(Duration::from_secs(合理默认值))` |
| `src/xcache/init.rs` (1 处) | `.unwrap_or(Duration::from_secs(300))` |
| doc tests (3 处) | 加 `.unwrap()` |

---

## 批次 E: functional options → 直接结构体

### 动机
Go 的 functional options 模式用 `Box<dyn FnOnce>` 模拟，有 heap allocation。
Rust 中直接传结构体更自然、零开销。

### 变更
1. `src/xhook/options.rs`: 删除 `OptionFn` 类型别名、`order()` 函数、`must_invoke_success()` 函数，保留 `HookOptions` 结构体 + Default
2. `src/xhook/mod.rs`: `before_start/before_stop` 签名从 `opts: Vec<OptionFn>` → `opts: HookOptions`

### 7 个调用方统一改法

| 原代码 | 新代码 |
|--------|--------|
| `vec![]` | `HookOptions::new()` |
| `vec![order(N)]` | `HookOptions::new().order(N)` |
| `vec![must_invoke_success(false), order(N)]` | `HookOptions::new().order(N).must_success(false)` |

| 文件 | 说明 |
|------|------|
| `src/xconfig/mod.rs` | register_hook |
| `src/xlog/mod.rs` | register_hook |
| `src/xlog/init.rs` | before_stop for flush |
| `src/xtrace/mod.rs` | 2 处 register_hook |
| `src/xhttp/mod.rs` | register_hook |
| `src/xorm/mod.rs` | 2 处 register_hook |
| `src/xcache/mod.rs` | 2 处 register_hook |
| `src/xhook/mod.rs` | 测试约 10 处 |

---

## 批次 F: 重命名 parse_config + serde 默认值 + 强类型

### F1: unmarshal_config → parse_config

```rust
// Before
pub fn unmarshal_config<T: DeserializeOwned>(key: &str) -> Result<T, String>

// After
pub fn parse_config<T: DeserializeOwned>(key: &str) -> Result<T, XOneError>
```

- 全部 10 处调用方 `unmarshal_config` → `parse_config`
- 大部分调用方已用 `.unwrap_or_default()` 或 `match`，类型变更透明

### F2: serde 默认值替代 merge_default

用自定义 `impl Default` 替代手动 merge 逻辑：

| 结构体 | 默认值 | 删除的函数 |
|--------|--------|-----------|
| `XLogConfig` | 实际默认值 | `config_merge_default()` |
| `AxumConfig` | host="0.0.0.0", port=8000 | `axum_config_merge_default` |
| `AxumSwaggerConfig` | schemes=["https","http"] | `axum_swagger_config_merge_default` |
| `ServerConfig` | version="v0.0.1" | `server_config_merge_default` |

调用方改为：`parse_config().unwrap_or_default()`

### F3: 强类型

**LogLevel 枚举**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace, Debug, #[default] Info, Warn, Error,
}
```
- `XLogConfig.level: String` → `level: LogLevel`

**端口类型**:
- `AxumConfig.port: i32` → `port: u16`
- `src/xserver/axum.rs`: 删除 `axum_config.port as u16`，直接用 `axum_config.port`

---

## 验证
每个批次完成后运行:
```bash
cargo test && cargo clippy -- -D warnings && cargo fmt
```
最终确认 217 测试全部通过。
