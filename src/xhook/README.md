# XHook - 生命周期钩子模块

提供应用生命周期管理，支持启动前（BeforeStart）和停止前（BeforeStop）钩子，支持排序执行、超时控制和 Panic 恢复。

## 核心特性

- **有序执行**：通过 `order` 参数控制执行顺序（从小到大）
- **Panic 恢复**：钩子中的 panic 会被自动捕获，避免进程崩溃
- **超时控制**：每个钩子独立超时（默认 5 秒），防止卡死
- **宏注册**：通过 `before_start!` / `before_stop!` 宏注册，自动记录调用位置

## API

```rust
use x_one::xhook::HookOptions;

// 最简用法：使用默认选项（order=100, timeout=5s）
x_one::before_start!(|| {
    println!("Initializing...");
    Ok(())
});

// 指定 order
x_one::before_start!(|| {
    println!("Early init...");
    Ok(())
}, HookOptions::with_order(50));

// 注册停止前钩子
x_one::before_stop!(|| {
    println!("Closing connections...");
    Ok(())
}, HookOptions::with_order(10));
```

## HookOptions

| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `order` | `i32` | `100` | 执行顺序，越小越先执行 |
| `must_invoke_success` | `bool` | `true` | 失败时是否中断后续 hook |
| `timeout` | `Duration` | `5s` | 单个 hook 执行超时 |

## 内置模块 Hook 顺序

### BeforeStart

| 模块 | Order |
|---|---|
| xconfig | 1 |
| xlog | 2 |
| xtrace | 3 |
| xhttp | 4 |
| xorm | 5 |
| xcache | 6 |
| 用户自定义 | 100（默认） |

### BeforeStop

| 模块 | Order |
|---|---|
| xtrace | 1 |
| xcache | 2 |
| xorm | 3 |
| 用户自定义 | 100（默认） |
| xlog guard | 100（最后关闭日志） |

> 用户自定义钩子建议使用 order >= 10（start）/ order >= 10（stop），避免与内置模块冲突。
