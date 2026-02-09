# Rust 编码规则

## 代码风格
- 遵循 rustfmt 默认格式化规则，不自定义 rustfmt.toml
- 所有代码必须通过 `cargo clippy -- -D warnings` 无警告
- 使用 Rust 2024 edition 特性

## 类型与安全
- 优先使用强类型和枚举，避免字符串魔法值
- 错误处理使用 `Result<T, E>`，禁止在生产代码中使用 `unwrap()` / `expect()`
- 测试代码中允许 `unwrap()`
- 使用 `thiserror` 或自定义 Error 枚举处理错误

## 命名规范
- 模块和变量：`snake_case`
- 类型和 trait：`PascalCase`
- 常量：`SCREAMING_SNAKE_CASE`
- 生命周期参数：简短有意义，如 `'a`, `'input`

## 文档
- 公开 API 必须有 `///` 文档注释
- 文档注释使用中文
- 包含用法示例的文档测试（`/// # Examples`）

## 模块组织
- 单一职责：每个模块只做一件事
- 公开接口最小化：默认私有，只暴露必要的 API
- 避免循环依赖

## mod.rs 规范
- `mod.rs` 只负责三件事：子模块声明（`pub mod`）、公开 API 重导出（`pub use`）、`register_hook()` 幂等注册
- 禁止在 `mod.rs` 中放置业务逻辑实现（struct 定义、方法实现、数据处理等）
- 全局状态（`OnceLock`/`static`）和对应的存储访问函数可以放在 `mod.rs` 中
- 测试辅助函数（`reset_config`、`set_config` 等）可以放在 `mod.rs` 中
- 宏定义（`#[macro_export]`）可以放在 `mod.rs` 中
- 业务逻辑应拆到对应的子模块：配置访问 → `accessor.rs`，服务器实现 → `server.rs`，客户端 API → `client.rs`
- 参考标杆：`xorm/mod.rs`（纯 mod 管理 + register_hook）