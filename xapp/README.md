# xapp —— 应用身份

应用名、版本（核心模块，`go get github.com/xiaoshicae/x-one` 就有）。

```go
name, version := xapp.Name(), xapp.Version()
```

## 配置

链路的 `service.name` / `service.version`、接口文档（XGinSwagger）的默认标题和版本都取自这里。

```yaml
App:
  Name: xone.demo.app      # 建议 team.system.app，默认空
  Version: v1.2.0          # 默认空
```

- 指标不读这一块。要在指标上区分应用，用 `XMetric.ConstLabels`（如 `app: xone.demo.app`）。
- 环境变量 `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME` 压过这里，优先级见 [xtrace「行为与实测」](../xtrace/README.md#行为与实测)。
- 代码里读：`xapp.Name()` / `xapp.Version()`。
