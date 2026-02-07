---
name: refactor
description: "在测试保护下安全重构代码"
user-invocable: true
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# 安全重构

在测试保护下安全重构：$ARGUMENTS

严格按以下步骤执行：

## 1. 安全检查
- 运行 `cargo test` 确认当前所有测试通过（如果不通过，先修复再重构）
- 记录当前测试数量作为基线

## 2. 分析
- 识别要重构的代码区域
- 说明重构目标（消除重复、改善命名、简化结构等）
- 确认重构**不改变外部行为**

## 3. 执行重构
- 每次只做一个小的重构步骤
- 每步之后立即运行 `cargo test` 确认测试仍然通过
- 如果测试失败，立即回退该步骤

## 4. 验证
- 运行 `cargo clippy -- -D warnings` 确认无警告
- 运行 `cargo fmt` 格式化
- 确认测试数量不少于基线（重构不应删除测试）
- 总结重构内容和改进效果