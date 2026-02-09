# 迁移说明

本项目已将 Go 风格命名全面替换为 Axum，并移除旧配置与 API 的兼容。

## 破坏性变更一览

### v0.1.2: 配置键迁移 `Server.Axum` → `XAxum`

Axum 配置已从 `Server` 的子级提升为独立的顶层配置键 `XAxum`，与 `XHttp`、`XTrace` 等模块保持一致。

#### 配置键
- `Server.Axum` → `XAxum`
- `Server.Axum.Swagger` → `XAxum.Swagger`

#### API 变更
- `xconfig::get_axum_config()` → `xaxum::load_config()`
- `xconfig::get_axum_swagger_config()` → `xaxum::load_swagger_config()`
- `xconfig::AxumConfig` → `xaxum::AxumConfig`
- `xconfig::AxumSwaggerConfig` → `xaxum::AxumSwaggerConfig`

#### 配置示例（新）

```yaml
Server:
  Name: "my-service"

XAxum:
  Host: "0.0.0.0"
  Port: 8000
  UseHttp2: false
  Swagger:
    Schemes: ["https", "http"]
```

### v0.1.1: Gin → Axum 命名统一

#### 配置键
- `Server.Gin` → `Server.Axum`（现已变为 `XAxum`）
- `Server.Gin.GinSwagger` → `Server.Axum.Swagger`（现已变为 `XAxum.Swagger`）

#### 配置结构与函数
- `GinConfig` → `AxumConfig`
- `GinSwaggerConfig` → `AxumSwaggerConfig`
- `get_gin_config()` → `get_axum_config()`（现已变为 `xaxum::load_config()`）
- `get_gin_swagger_config()` → `get_axum_swagger_config()`（现已变为 `xaxum::load_swagger_config()`）

#### 服务器类型与运行函数
- `GinServer` → `AxumServer`
- `GinTlsServer` → `AxumTlsServer`
- `run_gin()` → `run_axum()`
- `run_gin_tls()` → `run_axum_tls()`

#### 模块路径
- `xserver::gin` → `xaxum`
