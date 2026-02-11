---
name: perf
description: "对指定模块进行性能瓶颈扫描，定位热点代码并给出修复建议"
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Task
---

# 性能瓶颈分析

对指定模块进行性能瓶颈扫描，定位热点代码并给出修复建议。

使用方法: /perf <模块名>

示例:
- /perf xhttp
- /perf xorm
- /perf xlog

$ARGUMENTS

---

请对 `$ARGUMENTS` 模块进行性能瓶颈分析：

## 1. 运行 Benchmark（如果存在）

```bash
# 查找并运行已有的 benchmark
cargo bench --bench '*' 2>&1 || true
# 也检查 benches/ 目录
ls benches/ 2>/dev/null || true
```

如果存在 benchmark 结果，分析吞吐量和延迟指标。

## 2. 逐文件扫描性能瓶颈

读取 `src/$ARGUMENTS/` 目录下所有 `.rs` 文件（排除测试文件），逐一排查以下问题：

### 内存分配（高频问题）

- `Vec::new()` 在循环内重复创建，未使用 `Vec::with_capacity(n)` 预分配
- `String` 拼接使用 `format!()` 而非 `String::with_capacity()` + `push_str()`
- 频繁 `.clone()` 大结构体，应改用引用或 `Arc<T>`
- `to_string()` / `to_owned()` 在热路径上不必要地分配堆内存
- `Vec<u8>` 与 `String` 之间反复转换
- 返回 `Vec<T>` 而非迭代器，导致不必要的收集
- `Box<dyn Trait>` 可替换为枚举分发（避免动态分发+堆分配）

### 锁与并发

- `RwLock` 写锁作用域过大，可缩小临界区（尤其 `parking_lot::RwLock`）
- 读多写少场景未使用 `RwLock`，误用 `Mutex`
- 热路径上持有锁时执行 I/O 或耗时操作
- `Arc::clone()` 在循环内频繁调用，可提取到循环外
- 全局 `OnceLock<RwLock<T>>` 在高并发下的锁竞争
- 未使用 `AtomicBool` / `AtomicUsize` 替代简单标志位的锁

### 异步与 Tokio

- `tokio::spawn` 创建过多小任务，任务调度开销大于计算本身
- `.await` 在循环内逐个等待，应改用 `join!` / `FuturesUnordered` 并发
- 阻塞操作（文件 I/O、CPU 密集）未使用 `tokio::task::spawn_blocking`
- 异步函数内不必要的 `Box::pin` 或 `async move`
- `tokio::select!` 分支过多或含复杂逻辑

### 序列化与类型转换

- 热路径使用 `serde_json` / `serde_yaml`，可考虑零拷贝反序列化
- `as` 类型转换在循环内频繁执行（整数类型提升）
- `HashMap` 的 key 使用 `String` 而非 `&str`，导致查找时不必要的分配
- 正则表达式未预编译（`Regex::new()` 应放到 `OnceLock` 或 `lazy_static`）
- `format!()` 用于简单拼接，可用 `concat!` 或直接 `push_str`

### 集合与数据结构

- O(n²) 嵌套循环可优化为 O(n log n) 或 O(n)
- `Vec` 线性查找可改用 `HashMap` / `HashSet`
- 重复计算可缓存结果
- `HashMap` 未指定初始容量，频繁扩容 rehash
- 使用 `BTreeMap` 但不需要有序性，`HashMap` 更快
- `contains()` + `get()` 分两次查找，应使用 `entry()` API

### I/O 与网络

- HTTP 客户端未复用（每次请求创建新 `reqwest::Client`）
- 连接池参数未配置或不合理（`pool_max_idle_per_host`）
- 响应 body 未使用流式读取，大响应全量加载到内存
- 未使用 `BufReader` / `BufWriter` 缓冲文件 I/O
- 同步 I/O 阻塞异步 runtime

### 数据库（如果涉及 sqlx）

- N+1 查询（循环内逐条查询）
- 未使用 `query_as!` 编译期检查，运行时类型转换开销
- 连接池参数不合理（`max_connections` 过大或过小）
- 未使用批量插入/更新（`INSERT ... VALUES (...), (...)`）
- `SELECT *` 返回不需要的字段

## 3. 生成性能分析报告

以表格形式输出，按影响程度排列：

| 影响 | 文件:行号 | 瓶颈类型 | 问题描述 | 修复建议 | 预估收益 |
|------|-----------|----------|----------|----------|----------|
| 高 | xhttp/client.rs:58 | 内存分配 | 循环内未预分配 Vec | `Vec::with_capacity(len)` | 减少堆分配 |

影响分级：
- **高**：热路径上的问题，直接影响吞吐量或延迟
- **中**：非热路径但会在高负载下暴露
- **低**：微优化，仅在极端场景有收益

## 4. 给出优化优先级建议

根据发现的问题，给出优化的优先级建议：
1. 先修复哪些问题收益最大
2. 哪些可以快速修复（一行改动）
3. 哪些需要重构（需要评估风险）

## 注意事项

- 只做分析和建议，**不要直接修改代码**
- 如果用户需要修复，用 `AskUserQuestion` 确认后再动手
- 关注本项目特有的模式：`OnceLock<RwLock<T>>` 全局状态、Hook 初始化流程、`parking_lot` 锁