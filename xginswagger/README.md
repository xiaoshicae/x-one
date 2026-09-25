# xginswagger —— 接口文档

Swagger UI，单独一个 module。在路由里注册：

```go
xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
	xginswagger.Register(e, docs.SwaggerInfo)
}))
```

## 配置

单独一个 module，UI 资源不进普通服务。在路由里 `xginswagger.Register(e, docs.SwaggerInfo)`。

```yaml
XGinSwagger:
  Host: api.example.com    # 文档里显示的地址
  BasePath: /api/v1
  Title: ""                # 默认取 App.Name
  Description: ""
  Schemes: []              # 默认留空：沿用注解里的 @schemes，写了才覆盖
  URLPrefix: ""            # UI 挂载路径前缀，默认挂在 /swagger/*any；以 / 开头、不以 / 结尾
```

- 没写的字段一律沿用注解里的值，写了才覆盖；标题、版本默认取自 `App`，第一次访问文档时才填。
- `Register` 在 `Start` 之前任何时候调都行，读到的是最终配置。
