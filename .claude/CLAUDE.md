# x-one 项目规范

## 语言
- 所有对话、代码注释、文档使用简体中文
- commit message 使用英文

## 技术栈
- Rust edition 2024
- 库项目（lib crate）

## 开发方法
- 严格遵循 TDD 红-绿-重构循环（详见 rules/tdd.md）
- 编码规范详见 rules/rust-coding.md
- 测试规范详见 rules/testing.md

## 常用命令
```bash
cargo test              # 运行所有测试
cargo clippy            # 静态分析
cargo fmt               # 格式化
```

## 自定义 Skills
- `/tdd <功能>` - 执行一轮 TDD 红-绿-重构循环
- `/check` - 运行完整质量检查流水线
- `/new-feature <功能>` - 以 TDD 方式开发新功能
- `/refactor <目标>` - 在测试保护下安全重构
