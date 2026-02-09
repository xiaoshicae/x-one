# XUtil - 工具函数库

提供常用的基础工具函数，涵盖文件、JSON、重试、时长转换等。

## 功能列表

### 命令行 (Cmd)
- `get_config_from_args`: 从启动命令行参数中获取指定 key 的值。

### 时长转换 (Convert)
- `to_duration`: 将字符串 (如 `"1d"`, `"5m"`, `"1h30m"`) 转换为 `std::time::Duration`。

### 默认值 (DefaultValue)
- `default_if_empty`: 若值为零值则返回 fallback（借用版本）。
- `take_or_default`: 若值为零值则返回 fallback（所有权版本）。
- 基于 `IsZero` trait（`Default + PartialEq` blanket impl），自动覆盖 `String`、`Vec`、`Option`、数值、`bool` 等类型。

### 调试日志 (DebugLog)
- `info_if_enable_debug`: 调试模式下打印 info 日志。
- `warn_if_enable_debug`: 调试模式下打印 warn 日志。
- `error_if_enable_debug`: 调试模式下打印 error 日志。

### 环境变量 (Env)
- `enable_debug`: 判断是否启用了调试模式。

### 文件 (File)
- `file_exist`: 判断文件是否存在。
- `dir_exist`: 判断目录是否存在。

### JSON
- `to_json_string`: 序列化为紧凑 JSON。
- `to_json_string_indent`: 序列化为带缩进的 JSON。

### 重试 (Retry)
- `retry`: 同步函数重试（指数退避）。
- `retry_async`: 异步函数重试（指数退避）。

## 使用示例

```rust
use x_one::xutil;

fn main() {
    let duration = xutil::to_duration("1h30m").unwrap();

    if xutil::file_exist("config.yml") {
        println!("Config found");
    }

    let name = xutil::default_if_empty("", "default_name");
    assert_eq!(name, "default_name");

    // 数值零值也适用
    assert_eq!(xutil::take_or_default(0_u64, 100_u64), 100);
}
```
