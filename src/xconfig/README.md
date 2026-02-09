# XConfig - 配置文件解析模块

💡 负责配置文件的解析与加载，是其它模块的基础。支持 YAML 格式、环境变量替换、多环境 Profile 切换。

## 核心特性

- **多环境支持**：参考 Spring 设计，通过 `application-{profile}.yml` 区分不同环境。
- **层级覆盖**：启动参数 > 环境变量 > 配置文件。
- **自动搜索**：支持在当前目录、`./conf`、`./config` 等路径查找配置文件。

## 启用原则

### 1. 启用 Profile

优先级：启动参数 > 环境变量 > `application.yml` 配置。

- **启动参数**: `--server.profiles.active=dev`
- **环境变量**: `export SERVER_PROFILES_ACTIVE=prod`
- **配置文件**:
  ```yaml
  Server:
    Profiles:
      Active: test
  ```

### 2. 配置文件路径

优先级：启动参数 > 环境变量 > 默认路径搜索。

- **启动参数**: `--server.config.location=/etc/app.yml`
- **环境变量**: `export SERVER_CONFIG_LOCATION=/etc/app.yml`
- **默认路径**: `./application.yml` > `./conf/application.yml` > `./config/application.yml`

## 配置示例

```yaml
Server:
  Name: "my-service" # 服务名 (必填)
  Version: "v1.0.0"

  Profiles:
    Active: "dev"    # 激活 dev 环境配置 (application-dev.yml)
```

## 使用 Demo

```rust
use x_one::xconfig;

// 读取自定义配置
let my_val = xconfig::get_string("MyConfig.Key");
let my_int = xconfig::get_int("MyConfig.Count");

// 解析结构体
#[derive(serde::Deserialize)]
struct MyConfig {
    x: i32,
    y: String,
}
let config: MyConfig = xconfig::parse_config("MyConfig").unwrap();
```
