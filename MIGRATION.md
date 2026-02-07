# 迁移说明

本项目已将 Go 风格命名全面替换为 Auxm，并移除旧配置与 API 的兼容。

## 破坏性变更一览

### 1) 配置键
- `Server.Axum` → `Server.Auxm`
- `Server.Axum.GinSwagger` → `Server.Auxm.Swagger`

### 2) 配置结构与函数
- `GinConfig` → `AuxmConfig`
- `GinSwaggerConfig` → `AuxmSwaggerConfig`
- `get_gin_config()` → `get_auxm_config()`
- `get_gin_swagger_config()` → `get_auxm_swagger_config()`

### 3) 服务器类型与运行函数
- `AxumServer` → `AuxmServer`
- `AxumTlsServer` → `AuxmTlsServer`
- `run_axum()` → `run_auxm()`
- `run_axum_tls()` → `run_auxm_tls()`

### 4) 模块路径
- `xserver::axum` → `xserver::auxm`

## 配置示例（新）

```yaml
Server:
  Auxm:
    Host: "0.0.0.0"
    Port: 8000
    UseHttp2: false
    Swagger:
      Schemes: ["https", "http"]
```
