---
name: publish
description: "发布 crate 到 crates.io，发布前自动检查测试、增量覆盖率和工作区状态"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, AskUserQuestion
---

# 发布到 crates.io

## 执行步骤

### 1. 检查 Git 工作区

```bash
git status --porcelain
```

**如果工作区不干净**（有未提交的变更或未跟踪文件）：
- 输出当前 `git status` 给用户看
- 使用 `AskUserQuestion` 询问：
  - 选项 1：**取消发布**，先提交或清理
  - 选项 2：**继续发布**（不推荐，crates.io 打包的是已提交代码）
- 如果用户选择取消，立即终止

### 2. 检查版本 Tag

从 `Cargo.toml` 读取当前版本号，检查对应的 git tag 是否已存在。

```bash
# 读取 Cargo.toml 中的版本号
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo "当前版本：$VERSION"

# 检查 tag 是否已存在
git tag -l "v$VERSION"
```

**如果 tag `v<version>` 已存在**：
- 说明该版本已发布过，终止发布流程
- 提示用户：`tag v<version> 已存在，请先 bump 版本号再发布`

**如果 tag 不存在**：继续下一步。

### 3. 运行测试

```bash
cargo test 2>&1
```

**如果测试失败**：
- 输出失败的测试给用户看
- 终止发布流程，提示用户先修复

### 4. 运行 Clippy

```bash
cargo clippy -- -D warnings 2>&1
```

**如果 clippy 有警告**：
- 输出警告给用户看
- 终止发布流程，提示用户先修复

### 5. 检查增量代码覆盖率

检查相对于上一个 tag（或上一次发布版本）的增量代码覆盖率。

#### 5.1 确定基线

```bash
# 获取最近的 tag 作为基线
git describe --tags --abbrev=0 2>/dev/null || echo "NO_TAG"
```

如果没有 tag，以最近 10 次提交的 diff 作为增量范围。

#### 5.2 获取增量文件和行

```bash
# 获取相对于基线的变更文件（仅 src/ 下的 .rs 文件）
git diff <baseline>..HEAD --name-only -- 'src/**/*.rs'

# 获取具体变更行
git diff <baseline>..HEAD --unified=0 -- 'src/**/*.rs'
```

从 diff 中提取每个文件的新增行号范围（以 `+` 开头的行，排除文件头）。

#### 5.3 运行覆盖率

```bash
cargo tarpaulin --out json --output-dir /tmp/xone-cov --skip-clean 2>&1
```

如果 tarpaulin 未安装，提示用户：
```
cargo-tarpaulin 未安装，无法检查覆盖率。
安装命令：cargo install cargo-tarpaulin
```
然后使用 `AskUserQuestion` 询问：
- 选项 1：**跳过覆盖率检查**，继续发布
- 选项 2：**取消发布**，先安装 tarpaulin

#### 5.4 计算增量覆盖率

从 tarpaulin JSON 报告中，只统计增量行（步骤 4.2 提取的新增行）的覆盖情况：

```
增量覆盖率 = 增量行中被覆盖的行数 / 增量行总数 × 100%
```

**输出格式**：
```
增量覆盖率报告：
  变更文件数：N
  增量代码行：M
  已覆盖行：K
  增量覆盖率：XX.X%
```

**如果增量覆盖率 < 60%**：
- 列出覆盖率低的文件及未覆盖的行号
- 终止发布流程，提示用户补充测试
- 如果增量行数为 0（纯删除/纯配置变更），视为通过

### 6. 检查发布元数据

```bash
# 检查 Cargo.toml 必填字段
cargo package --list 2>&1 | head -5
```

确认以下字段存在且非空：
- `name`
- `version`
- `description`
- `license` 或 `license-file`
- `repository`

### 7. Dry-run 发布

```bash
cargo publish --dry-run 2>&1
```

**如果 dry-run 失败**：
- 输出错误信息给用户看
- 终止发布流程

### 8. 确认发布

使用 `AskUserQuestion` 最终确认：

显示以下信息并询问：
```
即将发布到 crates.io：
  包名：<name>
  版本：<version>
  描述：<description>

注意：发布后版本号不可重用，确认发布？
```

- 选项 1：**确认发布**
- 选项 2：**取消**

### 9. 执行发布

```bash
cargo publish 2>&1
```

**如果发布成功**：
- 创建 git tag：`git tag v<version>`
- 推送 tag：询问用户是否推送 `git push origin v<version>`

**如果发布失败**：
- 输出错误信息
- 常见错误提示：
  - `403`：运行 `cargo login` 重新登录
  - `verified email`：到 crates.io/settings/profile 验证邮箱
  - `already uploaded`：版本号已存在，需要 bump 版本

### 10. 输出结果

```
发布成功！
  包名：x-one
  版本：v<version>
  地址：https://crates.io/crates/x-one
```

## 用法

```
/publish              # 执行完整发布流程
```
