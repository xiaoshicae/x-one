# Crate 版本号规范

## 语义化版本 (SemVer)

版本号格式：`MAJOR.MINOR.PATCH`

| 字段 | 含义 | 何时递增 |
|------|------|----------|
| MAJOR | 主版本（不兼容） | API 破坏性变更（删除/重命名/改签名） |
| MINOR | 次版本（向后兼容） | 新增功能、新增模块 |
| PATCH | 补丁版本 | Bug 修复、性能优化、文档更新 |

## 0.x 阶段 vs 1.x+ 阶段

当前项目处于 **0.x 阶段**，规则有所不同：

### 0.x 阶段（当前）
- API 尚未稳定，**breaking change 只 bump minor**（0.1.0 → 0.2.0）
- 新功能 bump minor（0.1.0 → 0.2.0）
- Bug 修复 bump patch（0.1.0 → 0.1.1）
- 不需要 major bump（在 0.x 阶段 minor 承担 major 的角色）

### 1.x+ 阶段（稳定后）
- breaking change bump major（1.0.0 → 2.0.0）
- 新功能 bump minor（1.0.0 → 1.1.0）
- Bug 修复 bump patch（1.0.0 → 1.0.1）

## 提交类型与版本号映射

| 提交类型 | 版本影响 | 示例（0.x 阶段） | 示例（1.x+ 阶段） |
|----------|----------|-------------------|-------------------|
| feat | minor | 0.1.0 → 0.2.0 | 1.2.3 → 1.3.0 |
| feat! (breaking) | minor (0.x) / major (1.x+) | 0.1.0 → 0.2.0 | 1.2.3 → 2.0.0 |
| fix | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |
| refactor | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |
| docs | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |
| style | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |
| test | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |
| chore | patch | 0.1.0 → 0.1.1 | 1.2.3 → 1.2.4 |

**注意**：minor bump 时 patch 归零（0.2.3 → 0.3.0），major bump 时 minor 和 patch 都归零。

## crates.io 发布注意事项

### 发布前检查
- `Cargo.toml` 必填字段：`name`、`version`、`description`、`license`、`repository`
- `cargo publish --dry-run` 预检通过
- 工作区干净（无未提交变更）

### 发布后不可撤回
- crates.io 上的版本号**不可重用**，发布后无法覆盖
- 只能 yank（标记为不推荐），但已依赖的用户仍可下载
- 发布前务必确认版本号正确

### Cargo.lock
- lib crate 不提交 `Cargo.lock`（已在 .gitignore 中）
- bin crate 应提交 `Cargo.lock`

## Tag 命名规范

- 格式：`v<VERSION>`，如 `v0.2.7`
- Tag 与 `Cargo.toml` 中的 version 字段一一对应
- 先发布到 crates.io，成功后再打 tag 并推送

## 禁止事项

- **禁止**发布后重用版本号
- **禁止** tag 与 Cargo.toml 中的版本号不一致
- **禁止**在测试未通过时发布
- **禁止**在工作区不干净时发布（crates.io 打包的是已提交代码）