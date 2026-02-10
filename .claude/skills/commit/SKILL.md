---
name: commit
description: "创建规范的 Git 提交，自动更新版本号和 CHANGELOG"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Edit, AskUserQuestion
---

# 创建 Git 提交

## 执行步骤

### 1. 检查当前状态

```bash
# 查看工作目录状态
git status

# 查看暂存区和未暂存的变更
git diff --stat
git diff --cached --stat

# 查看最近的提交风格
git log --oneline -5
```

### 2. 运行质量检查

提交前必须通过以下检查：

```bash
# 格式检查（自动修复）
cargo fmt

# 静态分析
cargo clippy -- -D warnings

# 运行测试
cargo test
```

如果 clippy 或测试失败，使用 `AskUserQuestion` 询问用户：
- 选项 1：**取消提交**，先修复问题
- 选项 2：**跳过检查**（不推荐）

### 3. 确定提交类型和描述

**如果用户提供了提交信息**（$ARGUMENTS 非空）：解析类型和描述。

**否则**：根据变更内容自动分析，选择合适的类型。

**提交类型**：

| 类型 | 用途 | 版本影响 |
|------|------|----------|
| feat | 新功能 | minor（0.1.0 → 0.2.0） |
| fix | Bug 修复 | patch（0.1.0 → 0.1.1） |
| refactor | 重构 | patch |
| docs | 文档更新 | patch |
| style | 代码格式 | patch |
| test | 测试相关 | patch |
| chore | 构建/工具 | patch |

**破坏性变更**：如果提交描述中包含 BREAKING CHANGE 或类型后有感叹号（如 feat!:），
则 bump major（0.1.0 → 1.0.0）。但 0.x.y 阶段 breaking change 只 bump minor。

### 4. 自动更新版本号（语义化版本）

读取 `Cargo.toml` 中的当前版本号，根据提交类型自动 bump：

```
当前版本 → 提交类型 → 新版本
0.1.0   → feat     → 0.2.0（minor）
0.1.0   → fix      → 0.1.1（patch）
0.2.0   → feat!    → 0.3.0（0.x 阶段 breaking = minor）
1.0.0   → feat!    → 2.0.0（major）
1.2.3   → feat     → 1.3.0（minor，patch 归零）
1.2.3   → fix      → 1.2.4（patch）
```

**操作**：
1. 用 Edit 工具更新 `Cargo.toml` 中的 `version = "x.y.z"`
2. 运行 `cargo check` 确保版本更新不破坏编译（也会更新 Cargo.lock）

### 5. 更新 CHANGELOG

在 `README.md` 的 `## 更新日志` 区域，在已有条目**最前面**插入一行新条目：

```markdown
- **vX.Y.Z** (YYYY-MM-DD) - <简短描述>
```

**规则**：
- 每个版本一行，格式：`- **vX.Y.Z** (YYYY-MM-DD) - <描述>`
- 日期使用当天日期，格式 `YYYY-MM-DD`
- 如果新版本号与最近一条相同（连续多次同类型提交），合并到同一行
- 描述使用中文，简洁概括主要变更

### 6. 暂存变更

暂存所有相关文件，包括版本更新产生的 `Cargo.toml`、`Cargo.lock`、`README.md`。

**禁止暂存**：`.env`、`credentials`、`secret` 等敏感文件。

### 7. 创建提交

提交信息使用英文（项目规定）：

```bash
git commit -m "$(cat <<'EOF'
<type>: <description>

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

### 8. 验证

```bash
git status
git log --oneline -1
```

## 提交信息规范

**格式**：`<type>: <description>`

**规则**：
- 英文（项目规定）
- 描述简洁，聚焦「为什么」而非「改了什么」
- 破坏性变更在类型后加感叹号，如 feat!: redesign config API

## 用法

```
/commit                              # 自动分析变更并提交
/commit feat: add retry support      # 使用指定的提交信息
/commit fix: resolve connection leak # 指定 fix 类型
```