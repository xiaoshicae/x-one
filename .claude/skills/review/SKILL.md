# 模块代码审查

> 深度审查知识库见 `agents/code-reviewer.md` 和 `agents/security-reviewer.md`

对指定模块进行代码审查，分析潜在的优化点。

使用方法: /review <模块名>

示例:
- /review xhttp
- /review xorm

$ARGUMENTS

---

请对 `$ARGUMENTS` 模块进行代码审查，分析以下方面：

## 1. 执行静态分析

```bash
cargo clippy -- -D warnings 2>&1
cargo fmt --check 2>&1
cargo test $ARGUMENTS 2>&1
```

## 2. 逐文件审查

### 正确性
- 逻辑错误、边界条件、Option/Result 未处理
- 错误处理是否完整（生产代码禁止 `unwrap()`/`expect()`）

### 性能
- 是否有不必要的 clone/堆分配
- 锁的读写选择是否合理
- 是否有可以缓存的计算结果

### 并发安全
- 全局状态是否使用 `OnceLock<RwLock<T>>` + `parking_lot`
- 是否存在死锁风险（多锁嵌套）

### 代码质量
- 是否有重复代码可以提取
- 是否有 magic string/number 应该提取为常量
- 错误处理是否一致

### API 设计
- 是否缺少必要的公开 API
- 是否与其他模块保持一致（如使用 `xutil::to_duration`、`XOneError` 变体）

## 3. 生成审查报告

以表格形式输出发现的问题和建议，按优先级排列：
- P0：必须修复（错误、安全问题）
- P1：建议修复（性能、并发安全）
- P2：可选优化（代码质量、风格）