---
name: security-review
description: "对代码进行安全审查，检查 unsafe、依赖漏洞、硬编码密钥等"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob
---

# 安全审查

审查范围：$ARGUMENTS（默认为整个项目）

## 执行步骤

### 1. 依赖漏洞检查

```bash
# 检查已知漏洞（需要安装 cargo-audit）
cargo audit 2>&1 || echo "提示: 可用 cargo install cargo-audit 安装"
```

### 2. unsafe 代码审查

搜索所有 `unsafe` 使用，逐一审查：

- 是否有充分的 Safety 注释说明为什么是安全的
- 是否有更安全的替代方案
- 是否正确处理了所有不变量

注意：Rust 2024 中 `std::env::set_var/remove_var` 是 unsafe，测试中的使用是预期的。

### 3. 硬编码密钥检查

搜索以下模式：
- `password`、`secret`、`api_key`、`token`、`credential`
- Base64 编码的长字符串
- 类似密钥格式的字符串（如 `sk-`、`pk-`）

排除测试文件和文档中的示例值。

### 4. 输入验证检查

检查公开 API 的输入处理：
- 字符串输入是否有长度限制
- 数值输入是否有范围验证
- 配置值是否有合理的默认值和边界检查

### 5. 并发安全检查

- 全局状态访问是否正确使用 `OnceLock`/`RwLock`/`Mutex`
- 是否存在潜在的死锁风险（多锁嵌套）
- `parking_lot` 锁的使用是否正确

### 6. 错误信息泄露

- 错误消息是否暴露了内部实现细节
- 日志是否输出了敏感信息
- panic 消息是否安全

### 7. 生成安全报告

```
## 安全审查报告

### 依赖漏洞
- cargo audit 结果

### unsafe 使用
| 位置 | 用途 | 风险评估 |
|------|------|----------|
| file:line | 描述 | 低/中/高 |

### 硬编码密钥
- 是否发现

### 风险评级
- 🔴 高风险：需要立即修复
- 🟡 中风险：计划修复
- 🟢 低风险：建议改进

### 检查清单
- [ ] 无硬编码凭证
- [ ] unsafe 使用有充分说明
- [ ] 依赖无已知漏洞
- [ ] 全局状态并发安全
- [ ] 错误信息无敏感泄露
- [ ] 日志已脱敏
```

## 用法

```
/security-review               # 审查整个项目
/security-review src/xorm/     # 审查特定模块
/security-review unsafe        # 聚焦 unsafe 代码
```