# Rust 编码规则

## 代码风格
- 遵循 rustfmt 默认格式化规则，不自定义 rustfmt.toml
- 所有代码必须通过 `cargo clippy -- -D warnings` 无警告
- 使用 Rust 2024 edition 特性

## 类型与安全
- 优先使用强类型和枚举，避免字符串魔法值
- 错误处理使用 `Result<T, E>`，禁止在生产代码中使用 `unwrap()` / `expect()`
- 测试代码中允许 `unwrap()`
- 使用 `thiserror` 定义错误类型，统一用 `XOneError` 传播
- 并发锁使用 `parking_lot`（无需 unwrap，无 poisoning）

## 命名规范
- 模块和变量：`snake_case`
- 类型和 trait：`PascalCase`
- 常量：`SCREAMING_SNAKE_CASE`
- 生命周期参数：简短有意义，如 `'a`, `'input`

## 语言与文案
- 错误消息（`XOneError`、`tracing::info/warn/error` 等）：英文
- 代码注释（`//`、`///`、`//!`）：中文
- 文件级文档注释用 `//!`，不是 `///`（否则 clippy 报错）

## 文档
- 公开 API 必须有 `///` 文档注释（中文）
- 包含用法示例的文档测试（`/// # Examples`）
- Doc test 中引用本 crate 类型需要通过 `pub use` 重导出的路径

## 文件规模
- 单文件不超过 500 行，超出时拆分为子模块

## 文件内代码排列顺序
- 公开类型 → 公开函数 → 私有实现
- 所有 `pub` / `pub(crate)` 项必须排在 `fn`（私有）项之前，同一文件内不允许私有函数夹在公开函数之间

## 模块文件职责（详见 rules/architecture.md）
- `mod.rs`：模块声明 + pub use + register_hook()，禁止放业务逻辑
- `config.rs`：配置结构体
- `client.rs`：全局状态 + 对外查询 API
- `init.rs`：初始化/关闭逻辑

## Rust 2024 注意事项
- `std::env::set_var` / `remove_var` 是 `unsafe`，测试中需用 `unsafe {}` 包裹
- Clippy 常见问题：
  - 可 derive 的 Default 不要手写 `impl Default`
  - `map_err(|e| Variant(e))` → `map_err(Variant)`（redundant closure）
  - redundant closure 用函数指针代替

## 代码简化模式

编写新代码和审查现有代码时，优先采用以下模式减少重复和分配：

### 配置结构体
- 使用 `#[serde(default)]` 放在结构体级别，而非每个字段单独写 `default = "fn_name"`
- 默认值集中在 `impl Default` 中，不需要额外的 `fn default_xxx()` 辅助函数
- 参考：`xorm/config.rs`、`xcache/config.rs`、`xredis/config.rs`

```rust
// 推荐：结构体级别 serde(default) + 集中 Default impl
#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct XModuleConfig {
    #[serde(rename = "Timeout")]
    pub timeout: String,
}
impl Default for XModuleConfig {
    fn default() -> Self { Self { timeout: "30s".to_string() } }
}

// 避免：每个字段单独声明 default 函数
#[serde(rename = "Timeout", default = "default_timeout")]
pub timeout: String,
fn default_timeout() -> String { "30s".to_string() }
```

### 条件 Layer / 可选组件
- 使用 `Option<Layer>` 替代 if/else 三段式 subscriber 组装
- tracing-subscriber 原生支持 `.with(Option<Layer>)`，None 等同于无 Layer
- 需要类型擦除时用 `.boxed()` 转为 `Box<dyn Layer>`
- 参考：`xlog/init.rs`

### 重复逻辑合并
- 两个函数仅在"阶段名"或"是否中断"等参数上不同时，提取为带参数的通用函数
- 用 `&str` 传入阶段名（如 `"before start"` / `"before stop"`），用 `bool` 传入行为差异
- 参考：`xhook/manager.rs` 的 `invoke_hooks()`

### 信号处理
- 将 SIGINT/SIGTERM 合并为一个 `async fn wait_for_shutdown_signal() -> &'static str`
- 在 `tokio::select!` 中只需一个信号分支，返回信号名供日志使用
- 参考：`xserver/runner.rs`

### 避免不必要的堆分配
- 固定字符串路径用 `const` 常量，不用 `format!()` 拼接（如配置 key 路径）
- 命令行参数匹配用 `strip_prefix` 链替代 `format!("{key}=")` 临时 String
- 枚举转字符串用 `fn as_str(&self) -> &'static str` 替代 `.to_string()` 或 `.to_uppercase()`
- 参考：`xconfig/server_config.rs`、`xutil/cmd.rs`、`xlog/config.rs`

### 工具函数签名
- 文件路径参数用 `impl AsRef<std::path::Path>` 替代 `&str`，兼容 `&str`/`String`/`PathBuf`/`Path`
- 可序列化失败的操作提取 `fn xxx_or_empty(result, label)` 模式，统一错误日志
- 参考：`xutil/file.rs`、`xutil/json.rs`

### 重复代码块提取
- 相同逻辑出现 2 次以上时提取为私有辅助函数
- 函数命名体现"做什么"而非"在哪调用"（如 `collect_fields` 而非 `get_span_and_event_kv`）
- 参考：`xlog/otel_fmt.rs` 的 `collect_fields()`