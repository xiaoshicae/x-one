---
name: build-fix
description: "快速修复编译错误和 clippy 警告，做最小改动"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Edit
---

# 快速修复构建错误

修复范围：$ARGUMENTS（默认为整个项目）

## 修复原则

- ✅ 做最小改动
- ✅ 只修复错误和警告
- ❌ 不重构代码
- ❌ 不改变架构
- ❌ 不优化性能
- ❌ 不添加新功能

## 执行步骤

### 1. 收集所有错误

```bash
# 编译检查
cargo build 2>&1

# 静态分析
cargo clippy -- -D warnings 2>&1

# 格式检查
cargo fmt --check 2>&1
```

### 2. 按优先级修复

**修复顺序**：编译错误 → clippy 错误 → clippy 警告 → 格式问题

**常见 Rust 错误类型**：

| 错误类型 | 修复方式 |
|----------|----------|
| 未找到模块/类型 | 添加 use 导入或 mod 声明 |
| 类型不匹配 | 类型转换或修正签名 |
| 未使用变量 | 添加 `_` 前缀或删除 |
| 生命周期错误 | 添加/修正生命周期注解 |
| trait 未实现 | 添加 derive 或手动 impl |
| 借用检查错误 | 调整所有权或使用引用 |
| clippy 警告 | 按 clippy 建议修复 |

### 3. 自动格式化

```bash
cargo fmt
```

### 4. 验证修复

```bash
# 编译通过
cargo build 2>&1

# clippy 无警告
cargo clippy -- -D warnings 2>&1

# 测试通过
cargo test 2>&1
```

### 5. 报告结果

```
## 修复报告

### 已修复
- [文件:行号] 修复了什么

### 修改统计
- 修改文件数：N
- 修改行数：+M/-K

### 验证结果
- cargo build: ✅/❌
- cargo clippy: ✅/❌
- cargo test: ✅/❌
```

## 成功标准

- `cargo build` 返回 0
- `cargo clippy -- -D warnings` 无警告
- 改动行数最小

## 用法

```
/build-fix                 # 修复整个项目的构建错误
/build-fix src/xorm/       # 只关注特定模块
```