# XFlow - 流程编排模块

轻量级流程编排框架，提供顺序执行处理器、强/弱依赖模型和自动逆序回滚能力。

## 核心概念

| 概念 | 说明 |
|------|------|
| `Processor<T>` | 处理器 trait，定义步骤的处理和回滚逻辑 |
| `Step<T>` | 闭包式处理器，通过 Builder 模式快速创建步骤 |
| `Flow<T>` | 编排器，顺序执行处理器并管理回滚 |
| `Dependency` | 依赖强度：`Strong`（失败中断+回滚）/ `Weak`（失败跳过+继续） |
| `Monitor` | 监控回调接口，可选启用 |

## 快速开始

```rust
use x_one::xflow::{Flow, Step};

let flow = Flow::new("order")
    .step(Step::new("validate").process(|data: &mut Vec<String>| {
        data.push("validated".into());
        Ok(())
    }))
    .step(Step::new("save").process(|data: &mut Vec<String>| {
        data.push("saved".into());
        Ok(())
    }));

let mut data = Vec::new();
let result = flow.execute(&mut data);
assert!(result.success());
assert_eq!(data, vec!["validated", "saved"]);
```

## 强/弱依赖

```rust
use x_one::xflow::{Flow, Step};
use x_one::XOneError;

let flow = Flow::new("pipeline")
    // 强依赖（默认）：失败会中断流程并回滚
    .step(Step::new("core").process(|data: &mut i32| {
        *data += 10;
        Ok(())
    }))
    // 弱依赖：失败只记录，流程继续
    .step(Step::weak("optional").process(|_data: &mut i32| {
        Err(XOneError::Other("not critical".into()))
    }))
    .step(Step::new("final").process(|data: &mut i32| {
        *data *= 2;
        Ok(())
    }));

let mut data = 5;
let result = flow.execute(&mut data);
assert!(result.success());         // 整体成功
assert!(result.has_skipped_errors()); // 弱依赖的错误被记录
assert_eq!(data, 30);              // (5 + 10) * 2
```

## 自动逆序回滚

强依赖步骤失败时，已成功的步骤按**逆序**回滚：

```rust
use x_one::xflow::{Flow, Step};
use x_one::XOneError;

let flow = Flow::new("transaction")
    .step(Step::new("step1")
        .process(|data: &mut Vec<String>| {
            data.push("s1:done".into());
            Ok(())
        })
        .rollback(|data: &mut Vec<String>| {
            data.push("s1:rollback".into());
            Ok(())
        }))
    .step(Step::new("step2")
        .process(|_data: &mut Vec<String>| {
            Err(XOneError::Other("boom".into()))
        }));

let mut data = Vec::new();
let result = flow.execute(&mut data);
assert!(result.rolled());
assert_eq!(data, vec!["s1:done", "s1:rollback"]);
```

## 自定义 Processor

复杂场景可直接实现 `Processor` trait：

```rust
use x_one::xflow::{Processor, Dependency, Flow};
use x_one::XOneError;

struct DoubleProcessor;

impl Processor<i32> for DoubleProcessor {
    fn name(&self) -> &str { "double" }

    fn process(&self, data: &mut i32) -> Result<(), XOneError> {
        *data *= 2;
        Ok(())
    }

    fn rollback(&self, data: &mut i32) -> Result<(), XOneError> {
        *data /= 2;
        Ok(())
    }
}

let flow = Flow::new("calc").step(DoubleProcessor);
let mut data = 5;
flow.execute(&mut data);
assert_eq!(data, 10);
```

## 监控

默认**不启用**监控（零开销）。启用后可在步骤执行/回滚/流程完成时接收回调：

```rust
use x_one::xflow::Flow;

// 方式一：使用内置 DefaultMonitor（tracing 日志）
let flow = Flow::<()>::new("monitored").enable_monitor();

// 方式二：自定义 Monitor（自动启用）
// let flow = Flow::<()>::new("monitored").monitor(my_monitor);
```

自定义 `Monitor` 需实现 trait，回调方法包含 `Duration` 耗时和 `Option<&XOneError>` 错误信息。

## 执行结果

`ExecuteResult` 提供完整的执行状态：

| 方法 | 说明 |
|------|------|
| `success()` | 流程是否成功（无 Strong 错误） |
| `err()` | 主错误（Strong 依赖失败） |
| `has_skipped_errors()` | 是否有 Weak 依赖跳过的错误 |
| `skipped_errors()` | Weak 跳过错误列表 |
| `rolled()` | 是否触发了回滚 |
| `has_rollback_errors()` | 回滚过程中是否有错误 |

## 特性

- **panic 安全**：自动捕获处理器中的 panic，不会导致流程崩溃
- **零开销监控**：默认关闭，不启用时无 `Instant::now()` 调用
- **类型安全**：泛型 `T` 在编译期确保数据类型一致
- **独立模块**：不依赖框架其他模块（无 Hook、无配置），可独立使用
