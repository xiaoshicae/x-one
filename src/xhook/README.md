# XHook - 生命周期钩子模块

💡 提供应用生命周期管理，支持启动前 (`BeforeStart`) 和停止前 (`BeforeStop`) 钩子，支持排序、依赖管理和优雅停机。

## 核心特性

- **有序执行**：支持通过 `order` 参数控制钩子执行顺序。
- **优雅停机**：集成信号监听，在服务退出前执行资源清理（如关闭 DB 连接、刷新日志）。
- **Panic 恢复**：钩子执行过程中的 panic 会被自动捕获，避免进程意外崩溃。
- **超时控制**：支持配置 Shutdown 超时时间，防止清理逻辑卡死。

## 配置

```yaml
# 可通过代码配置超时时间，暂无 YAML 配置项
```

## API 接口

```rust
use x_one::xhook::HookOptions;

// 最简用法：仅传函数，使用默认选项
x_one::before_start!(|| {
    println!("Initializing cache...");
    Ok(())
});

// 指定选项：控制执行顺序
x_one::before_start!(|| {
    println!("Initializing cache...");
    Ok(())
}, HookOptions::with_order(10));

// 注册停止前钩子
x_one::before_stop!(|| {
    println!("Closing database...");
    Ok(())
}, HookOptions::with_order(1));
```

## 执行顺序

1.  **BeforeStart**: 按 `order` **从小到大** 执行。
2.  **BeforeStop**: 按 `order` **从小到大** 执行（通常用于逆序关闭，建议手动控制 order）。

> 注意：框架内部模块（Config, Log, Trace 等）占用了 order 1-10 的位置，用户自定义钩子建议从 100 开始。