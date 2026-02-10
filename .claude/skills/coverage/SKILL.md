---
name: coverage
description: "运行测试覆盖率分析，生成覆盖率报告"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob
---

# 测试覆盖率分析

## 执行步骤

### 1. 检查工具是否安装

```bash
cargo tarpaulin --version 2>&1 || echo "提示: 可用 cargo install cargo-tarpaulin 安装"
```

### 2. 运行覆盖率分析

**使用 cargo-tarpaulin**（推荐）：

```bash
cargo tarpaulin --out stdout --skip-clean 2>&1
```

**如果 tarpaulin 不可用**，使用基础统计：

```bash
# 运行测试并统计
cargo test 2>&1 | tail -20

# 统计测试数量分布
echo "=== 测试分布 ==="
echo "集成测试文件："
find tests/ -name "*.rs" -exec grep -l "#\[test\]\|#\[tokio::test\]" {} \;

echo "单元测试模块："
find src/ -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \;

echo "文档测试："
find src/ -name "*.rs" -exec grep -l "/// ```" {} \;
```

### 3. 分析覆盖率

如果有 tarpaulin 输出，分析各模块覆盖率：
- 识别覆盖率低于 60% 的模块
- 识别完全未覆盖的公开函数
- 高亮关键路径的覆盖情况

### 4. 生成报告

```
## 覆盖率报告

### 总体覆盖率
- 行覆盖率：XX%
- 测试总数：N（单元 M + 集成 K + 文档 J）

### 模块覆盖率
| 模块 | 覆盖率 | 状态 |
|------|--------|------|
| xutil | 85% | ✅ |
| xorm | 45% | ⚠️ 低于 60% |

### 未覆盖的关键函数
- `module::function` - 建议添加测试

### 建议
1. 优先补充覆盖率最低的模块
2. 关键路径（init/shutdown）需要 100% 覆盖
```

## 用法

```
/coverage                  # 运行全量覆盖率
```