# XUtil - 工具函数库

💡 提供常用的基础工具函数，涵盖网络、文件、JSON 处理等。

## 功能列表

### 网络 (Net)
- `get_local_ip`: 获取本机局域网 IP。
- `extract_real_ip`: 从请求 Header 或地址字符串中提取真实 IP，过滤 IPv6 括号。
- `validate_ip`: 校验 IP 格式。

### 文件 (File)
- `file_exist`: 判断文件是否存在。
- `dir_exist`: 判断目录是否存在。

### JSON
- `to_json_string`: 序列化为紧凑 JSON。
- `to_json_string_indent`: 序列化为带缩进的 JSON。

### 重试 (Retry)
- `retry`: 同步函数重试。
- `retry_async`: 异步函数重试。

### 时间/单位转换 (Convert)
- `to_duration`: 将字符串 (如 "1d", "5m") 转换为 `std::time::Duration`。

## 使用 Demo

```rust
use x_one::xutil;

fn main() {
    let ip = xutil::get_local_ip().unwrap();
    let duration = xutil::to_duration("1h30m").unwrap();
    
    if xutil::file_exist("config.yml") {
        println!("Config found");
    }
}
```