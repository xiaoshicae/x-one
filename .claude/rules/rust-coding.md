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

## 文件内代码排列顺序
公开类型 → 公开函数 → 私有实现

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