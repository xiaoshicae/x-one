---
name: pr
description: "创建 Pull Request，自动生成标题和描述"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, AskUserQuestion
---

# 创建 Pull Request

目标分支: ${ARGUMENTS:-main}

## 执行步骤

### 1. 检查当前状态

```bash
# 当前分支
git branch --show-current

# 未提交的变更
git status

# 与目标分支的差异
git log origin/<target-branch>..HEAD --oneline
git diff origin/<target-branch>..HEAD --stat
```

如果有未提交的变更，提示用户先执行 `/commit`。

### 2. 运行质量检查

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

如果检查未通过，使用 `AskUserQuestion` 询问是否继续。

### 3. 推送分支

```bash
git push -u origin <current-branch>
```

### 4. 分析变更

分析所有 commit（不仅是最后一个），总结变更内容：
- 新增/修改了哪些模块
- 主要变更点
- 测试覆盖情况

### 5. 展示 PR 预览

向用户展示将要创建的 PR 内容：

```
📋 PR 预览
目标分支：<target-branch>
当前分支：<current-branch>

📌 标题
<简短描述，不超过 70 字符>

📝 描述
## Summary
- 变更点 1
- 变更点 2

## Test plan
- [ ] 测试项 1
- [ ] 测试项 2

🤖 Generated with Claude Code
```

### 6. 用户确认

使用 `AskUserQuestion` 确认：
- 选项 1：确认创建
- 选项 2：修改内容
- 选项 3：取消

### 7. 创建 PR

```bash
gh pr create --base <target-branch> --title "<标题>" --body "$(cat <<'EOF'
## Summary
- 变更点

## Test plan
- [ ] 测试项

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

### 8. 返回 PR URL

创建成功后返回 PR 链接。

## 用法

```
/pr              # PR 到 main（默认）
/pr develop      # PR 到 develop 分支
```