# xapp —— 应用身份

`XApp` 块写应用本身的事：叫什么、什么版本、默认跑在哪个环境、配置从哪些文件来。
链路的 `service.name` / `service.version`、接口文档的默认标题和版本都取自这里，只配一次。

## 快速上手

```yaml
# conf/application.yml
XApp:
  Name: shop.order.api
  Version: v1.2.0
  Profiles: ${APP_ENV:dev}    # 默认激活的 profile
  Import: [common/log.yml]    # 再读哪些文件
```

```go
slog.Info("build info", "app", xapp.Name(), "version", xapp.Version())
```

## 重点

- **跟着框架一起来**，不用 import xapp；`Profiles`、`Import` 由配置加载本身读，`Name`、`Version` 由 xapp 读。
- 指标不读这一块：要在指标上区分应用，用 `XMetric.ConstLabels`。
- 环境变量 `OTEL_SERVICE_NAME` / `OTEL_RESOURCE_ATTRIBUTES` 压过这里的 `Name` / `Version`。
- 名字建议写成 `team.system.app`。

## 配置

```yaml
XApp:
  Name: xone.demo.app      # 建议 team.system.app，默认空
  Version: v1.2.0          # 默认空
  Profiles: dev            # 默认激活的 profile：一个名字、"prod,eu" 或 [prod, eu]；只能写在主配置文件里
  Import:                  # 再读哪些文件：一个写字符串，多个写列表；optional: 前缀表示可以没有
    - common/log.yml
```

- `Name`、`Version`：链路的 `service.name` / `service.version`、接口文档（XGinSwagger）的默认标题和版本都取自这里。
  指标不读，要在指标上区分应用用 `XMetric.ConstLabels`（如 `app: xone.demo.app`）。
  环境变量 `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME` 压过这里，优先级见 [xtrace「行为与实测」](../xtrace/README.md#行为与实测)。
  环境文件里可以覆盖，比如 `application-prod.yml` 换一个 `Name`。
- `Profiles`：`--profile` / `XONE_PROFILE` 给了就不看这一项（替换，不是追加）。规则见
  [docs/config.md「Profiles」](../docs/config.md#profiles--按环境分文件)。
- `Import`：相对路径按写着它的那个文件所在的目录解析，引进来的压过引它的；被引进来的文件里可以再写 `Import`，
  但不能写 `Profiles`。规则见 [docs/config.md「Import」](../docs/config.md#import--引入别的配置文件)。
- 这两项由配置加载本身读取，读完就从 `XApp` 里拿走，xapp 看到的只有 `Name`、`Version`。
- 一级的 `App`、`Import`、`Profiles` 是 v0.1.0 的写法，现在启动失败并说明怎么改。
- 代码里读：`xapp.Name()` / `xapp.Version()`。
