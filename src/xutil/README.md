# XUtil - 工具函数库

提供常用的基础工具函数，涵盖文件、JSON、重试、时长转换、默认值等。

## 功能列表

### 命令行 (cmd)
- `get_config_from_args(key) -> Option<String>`：从启动命令行参数中获取指定 key 的值

### 时长转换 (convert)
- `to_duration(s) -> Option<Duration>`：将人类可读字符串转换为 `Duration`，基于 [humantime](https://github.com/tailhook/humantime)

### 默认值 (default_value)
- `default_if_empty(value, fallback) -> &T`：若值为零值则返回 fallback（借用版本）
- `take_or_default(value, fallback) -> T`：若值为零值则返回 fallback（所有权版本）
- 基于 `IsZero` trait（`Default + PartialEq` blanket impl），自动覆盖 `String`、`Vec`、`Option`、数值、`bool` 等类型

### 调试日志 (debug_log)
- `info_if_enable_debug(msg)`：调试模式下打印 info 日志
- `warn_if_enable_debug(msg)`：调试模式下打印 warn 日志
- `error_if_enable_debug(msg)`：调试模式下打印 error 日志

### 环境变量 (env)
- `enable_debug() -> bool`：判断是否启用了调试模式（`SERVER_ENABLE_DEBUG=true`）

### 文件 (file)
- `file_exist(path) -> bool`：判断文件是否存在
- `dir_exist(path) -> bool`：判断目录是否存在

### JSON (json)
- `to_json_string(value) -> String`：序列化为紧凑 JSON
- `to_json_string_indent(value) -> String`：序列化为带缩进的 JSON

### 重试 (retry)
- `retry(f, max_retries, ...) -> Result<T, E>`：同步函数重试，基于 [backon](https://github.com/Xuanwo/backon) 指数退避
- `retry_async(f, max_retries, ...) -> Result<T, E>`：异步函数重试

## 使用示例

```rust
use x_one::xutil;
use std::time::Duration;

// 时长转换（返回 Option）
let d = xutil::to_duration("1h30m");   // Some(Duration 5400s)
let d = xutil::to_duration("invalid"); // None

// 文件检查
if xutil::file_exist("config.yml") {
    println!("Config found");
}

// 默认值（零值判断）
let name = xutil::default_if_empty("", "default_name");
assert_eq!(name, "default_name");

assert_eq!(xutil::take_or_default(0_u64, 100_u64), 100);
assert_eq!(xutil::take_or_default(42_u64, 100_u64), 42);

// JSON 序列化
let json = xutil::to_json_string(&vec![1, 2, 3]);
```
