---
name: test
description: "运行测试，支持指定模块或运行所有测试"
user-invocable: true
allowed-tools: Bash
---

# 运行测试

## 执行步骤

### 确定测试范围

**如果 $ARGUMENTS 非空**（指定了模块或测试名）：

```bash
cargo test $ARGUMENTS 2>&1
```

**如果 $ARGUMENTS 为空**（运行所有测试）：

```bash
cargo test 2>&1
```

### 输出结果

报告测试结果，包括通过/失败数量。如果有失败的测试，列出失败的测试名和错误信息。

## 用法

```
/test                  # 运行所有测试
/test xhttp            # 运行 xhttp 模块的测试
/test xorm::init       # 运行 xorm 模块 init 子模块测试
/test test_parse_config # 运行匹配名称的测试
```