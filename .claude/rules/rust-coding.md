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