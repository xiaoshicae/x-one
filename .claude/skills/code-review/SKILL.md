---
name: code-review
description: "对代码变更进行全面审查，包括静态分析、安全检查和改进建议"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob
---

# 代码审查

审查范围：$ARGUMENTS（默认为当前分支所有未提交的变更）

## 执行步骤

### 1. 收集变更

```bash
# 查看变更文件列表
git diff --name-only
git diff --cached --name-only

# 如果指定了 commit 范围（如 HEAD~3）
git diff <range> --name-only
```

### 2. 静态分析

```bash
# Clippy 检查
cargo clippy -- -D warnings 2>&1

# 格式检查
cargo fmt --check 2>&1
```

### 3. 逐文件审查

对每个变更的 `.rs` 文件，检查以下方面：

**正确性**：
- 错误处理是否完整（是否有遗漏的 `unwrap()`）
- 并发安全（全局状态访问是否正确加锁）
- 生命周期和所有权是否正确

**设计**：
- 是否遵循项目架构规范（mod.rs/config.rs/client.rs/init.rs 职责分离）
- 公开 API 是否有 `///` 文档注释
- 命名是否清晰、符合 Rust 惯例

**性能**：
- 是否有不必要的 clone/copy
- 是否有不必要的堆分配
- 异步代码是否正确使用

**安全**：
- 是否有 unsafe 代码未做充分说明
- 是否有硬编码密钥/凭证
- 输入验证是否充分

### 4. 运行测试

```bash
cargo test 2>&1
```

### 5. 生成审查报告

按优先级分类输出：

```
## 代码审查报告

### 🔴 关键问题（必须修复）
- [文件:行号] 问题描述

### 🟡 警告（应该修复）
- [文件:行号] 问题描述

### 🟢 建议（考虑改进）
- [文件:行号] 问题描述

### ✅ 检查通过项
- clippy: ✅/❌
- fmt: ✅/❌
- test: ✅/❌ (N passed, M failed)

### 总结
<整体评价和优先修复建议>
```

## 用法

```
/code-review               # 审查当前所有未提交变更
/code-review HEAD~3        # 审查最近 3 次提交
/code-review src/xorm/     # 审查特定目录
```