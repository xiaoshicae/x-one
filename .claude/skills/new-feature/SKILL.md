---
name: new-feature
description: "以 TDD 方式开始实现新功能，从需求分析到完整实现"
user-invocable: true
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# 以 TDD 方式实现新功能

以 TDD 方式实现：$ARGUMENTS

按以下流程执行：

## 1. 需求分析
- 分析功能需求，拆解为最小可测试的行为单元
- 列出要实现的测试用例清单（从简单到复杂）
- 向我展示计划，等待确认

## 2. 逐个 TDD 循环
对每个行为单元，严格执行红-绿-重构循环：
- **红**：写测试 → `cargo test` 确认失败
- **绿**：最少代码 → `cargo test` 确认通过
- **重构**：优化 → `cargo test` + `cargo clippy` 确认无问题

## 3. 收尾
- 确保公开 API 都有 `///` 文档注释
- 运行 `cargo fmt` 格式化
- 运行完整检查：`cargo clippy -- -D warnings && cargo test`
- 总结实现的功能和对应的测试覆盖