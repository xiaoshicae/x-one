# xginswagger —— 接口文档

把 Swagger UI 挂到 xgin 上。单独一个 module：UI 资源会编进二进制，不该让每个线上服务都背上。

## 快速上手

先用 [swag](https://github.com/swaggo/swag) 从注解生成 `docs` 包（`swag init`），然后在路由里注册：

```go
package main

import (
	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xgin"
	"github.com/xiaoshicae/x-one/xginswagger"

	"example.com/shop/docs" // swag init 生成的
)

func main() {
	xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
		xginswagger.Register(e, docs.SwaggerInfo) // 默认挂在 /swagger/*any
		// ……业务路由
	}))
}
```

```yaml
# conf/application.yml（可选）
XGinSwagger:
  BasePath: /api/v1
```

打开 `http://localhost:8080/swagger/index.html`（实际路径是 `xginswagger.URL()`）。

## 重点

- 没写的字段一律沿用注解里的值，写了才覆盖；标题、版本默认取自 `App`。
- 挂载前缀用 `URLPrefix`（以 `/` 开头、不以 `/` 结尾），写错启动失败。

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
