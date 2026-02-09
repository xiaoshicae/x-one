# XConfig - 配置管理模块

负责配置文件的解析与加载，是其它模块的基础。支持 YAML 格式、环境变量替换、多环境 Profile 切换。

## 核心特性

- **多环境支持**：通过 `application-{profile}.yml` 区分不同环境
- **层级覆盖**：启动参数 > 环境变量 > 配置文件
- **自动搜索**：在 `./`、`./conf/`、`./config/` 等路径查找配置文件
- **环境变量占位符**：支持 `${VAR:-default}` 语法

## Profile 启用

优先级：启动参数 > 环境变量 > 配置文件。

- **启动参数**：`--server.profiles.active=dev`
- **环境变量**：`export SERVER_PROFILES_ACTIVE=prod`
- **配置文件**：
  ```yaml
  Server:
    Profiles:
      Active: test
  ```

## 配置文件路径

优先级：启动参数 > 环境变量 > 默认路径搜索。

- **启动参数**：`--server.config.location=/etc/app.yml`
- **环境变量**：`export SERVER_CONFIG_LOCATION=/etc/app.yml`
- **默认路径**：`./application.yml` > `./conf/application.yml` > `./config/application.yml`

## 配置示例

```yaml
Server:
  Name: "my-service"    # 服务名（建议填写，多个模块依赖此值）
  Version: "v1.0.0"
  Profiles:
    Active: "dev"       # 激活 dev 环境（加载 application-dev.yml）
```

## 使用

```rust
use x_one::xconfig;

// 读取基础类型
let name = xconfig::get_string("Server.Name");
let port = xconfig::get_int("XAxum.Port");
let debug = xconfig::get_bool("Server.Debug");
let rate = xconfig::get_float64("Config.Rate");

// 读取字符串列表
let hosts = xconfig::get_string_slice("Config.Hosts");

// 检查 key 是否存在
if xconfig::contain_key("XLog") {
    // ...
}

// 解析为自定义结构体
#[derive(serde::Deserialize)]
struct MyConfig {
    x: i32,
    y: String,
}
let config: MyConfig = xconfig::parse_config("MyConfig").unwrap();
```
