---
name: check
description: "运行完整质量检查流水线：格式、静态分析、测试、文档"
user-invocable: true
allowed-tools: Bash, Read
---

# 完整质量检查

对项目运行完整的质量检查流水线，依次执行以下步骤，每步报告结果：

1. **格式检查**：`cargo fmt --check`
   - 如果有格式问题，自动运行 `cargo fmt` 修复并报告修改了哪些文件

2. **静态分析**：`cargo clippy -- -D warnings`
   - 如果有 clippy 警告，逐一修复

3. **全量测试**：`cargo test`
   - 报告测试通过/失败数量

4. **文档检查**：`cargo doc --no-deps 2>&1`
   - 检查是否有文档警告

最后输出一个总结表格，标明每项检查的通过/失败状态。