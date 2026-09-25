# xmetric —— 指标

Prometheus 指标：原生的 `*prometheus.Registry` 用 `xmetric.Registry()` 取，日常打点有一组免样板的快捷方法。
指标端点由 xgin 自动挂在 `/metrics`。

## 快速上手

```go
package order

import (
	"context"
	"time"

	"github.com/xiaoshicae/x-one/xmetric"
)

func Place(ctx context.Context, o *Order) (err error) {
	start := time.Now()
	defer func() {
		status := "ok"
		if err != nil {
			status = "error"
		}
		xmetric.CounterInc("orders_total", xmetric.T("status", status))
		xmetric.ObserveDuration("order_place", time.Since(start), xmetric.T("status", status)) // 导出为 order_place_seconds
	}()
	return save(ctx, o)
}

// 标签不随结果变时更短：defer xmetric.Timer("order_query")()
```

```yaml
# conf/application.yml（可选）
XMetric:
  Namespace: shop          # 所有指标名前加 shop_
  ConstLabels:
    env: "${ENV:dev}"      # 附加到所有指标上
```

## 重点

- **名字写错读配置时就失败**：`Namespace` 和 `ConstLabels` 的 key 只收字母、数字、下划线，不以数字开头
  （client_golang 不报错，会悄悄把 `my-app` 导出成 `my_app_…`）。见[「行为与实测」](#行为与实测)。
- **桶要严格递增**，写了就整体替换默认值；空列表 `[]` 不是「用默认」，直接失败。
- `ConstLabels` 不能用 `le`、`quantile`、`version` 和框架指标自己的标签名（`method`、`status`、`route`、`name`……）。
- 耗时指标自动补 `_seconds` 后缀，桶是 `XMetric.HistogramBuckets`。
- 不用 xgin 的服务自己挂 `xmetric.Handler()`。框架自带的指标见 [observability.md「指标」](../docs/observability.md#指标)。

## 配置

```yaml
XMetric:
  Namespace: myapp         # 指标名前缀，默认无
  ConstLabels:             # 附加到所有指标上（含 go_* / process_*），默认无
    env: "${ENV:dev}"
  HTTPDurationBuckets: [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]  # 出入站 HTTP 耗时（秒），此为默认
  HistogramBuckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]           # 快捷方法建的直方图，即 prometheus.DefBuckets
  GoMetrics: true          # Go 运行时指标，默认开
  ProcessMetrics: true     # 进程指标，默认开
  LogErrorMetric: true     # Error 级别日志计入 log_errors_total，默认开（需配合 xlog）
```

- 两组桶不写就是默认值，写了就整体替换；空列表 `[]` 不是「用默认」，读配置时失败。桶必须严格递增。
- `Namespace` 和 `ConstLabels` 的 key 只收「字母、数字、下划线，不以数字开头」；`le`、`quantile`、`version`
  和框架指标自己的变量标签（`level`、`caller`、`method`、`status`、`route`、`host`、`name`）不能用作 `ConstLabels` 的 key。
  都在读配置时失败，理由见 [「行为与实测」](#行为与实测)。
- 指标端点由 xgin 挂（`XGin.MetricPath`）；不用 xgin 的服务自己挂 `xmetric.Handler()`。框架自带的指标见
  [observability.md「指标」](../docs/observability.md#指标)。

## 行为与实测

实测环境和跨模块的总表见 [`docs/behavior.md`](../docs/behavior.md)。

client_golang v1.24.1。

| 写法 | 不拦的话（实测） |
|---|---|
| 桶不是严格递增（`[1, 0.5, 2]`、`[1, 1]`），或含 NaN / Inf | 通过启动，第一次 `HistogramObserve` 时在业务请求里 panic |
| 桶写成空列表 `[]` | 不是「用默认」：被 Prometheus 悄悄换成它自己的 `DefBuckets`（HTTP 那组少了 1ms 一档） |
| `Namespace` 或 `ConstLabels` 的 key 不合规 | 不报错，导出时转义：`my-app` 导出成 `my_app_…`，`1app` 成 `_app_…`，中文成一串下划线 |
| `ConstLabels` 的 key 是 `le` / `quantile` | `le` 通过启动，第一次 `HistogramObserve` 时 panic；`quantile` 让建 Summary 当场 panic |
| `ConstLabels` 的 key 撞上框架自带指标的变量标签 | 撞名的那个注册失败，只打一条错误日志，那组指标一个都导不出去 |
| `ConstLabels` 的 key 是 `version` | `go_info` 自带常量标签 `version`，client_golang 拒绝注册整组 Go 运行时指标（`attempted wrapping with already existing label name "version"`） |

所以这些在读配置时就失败。

## 可观测

### 指标

| 指标 | 类型 | 标签 | 来源 |
|---|---|---|---|
| `log_errors_total` | counter | `level`、`caller` | xmetric，`XMetric.LogErrorMetric`，需要 xlog |
| `go_*` / `process_*` | —— | —— | `XMetric.GoMetrics` / `ProcessMetrics` |

业务打点：`xmetric.CounterInc("orders_total", xmetric.T("status", "ok"))`、`defer xmetric.Timer("handle_order")()`
（耗时指标自动补 `_seconds` 后缀，桶是 `XMetric.HistogramBuckets`）；要完整控制就用 `xmetric.Registry()` 拿原生的
`*prometheus.Registry`。

指标名的前缀、常量标签和几条通用规则见 [`docs/observability.md`「指标」](../docs/observability.md#指标)。
