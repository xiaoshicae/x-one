# xflow —— 流程编排

把一串步骤编排成一个流程（核心模块）：强依赖的步骤失败时，已经做过的步骤逆序回滚。每一步只写 `Process` 和 `Rollback`。

## 快速上手

```go
package order

import (
	"context"
	"log/slog"

	"github.com/xiaoshicae/x-one/xflow"
)

// Order 贯穿整个流程的数据：入参、各步的中间结果都放在这里
type Order struct {
	ID       string
	SKU      string
	Amount   int64
	chargeID string // 扣款成功后记下，回滚时退款用
}

type reserveStock struct{}

func (reserveStock) Process(ctx context.Context, o *Order) error  { return inventory.Reserve(ctx, o.SKU, 1) }
func (reserveStock) Rollback(ctx context.Context, o *Order) error { return inventory.Release(ctx, o.SKU, 1) }

type charge struct{}

func (charge) Process(ctx context.Context, o *Order) (err error) {
	o.chargeID, err = payment.Charge(ctx, o.ID, o.Amount)
	return err
}

func (charge) Rollback(ctx context.Context, o *Order) error {
	if o.chargeID == "" {
		return nil // 没扣成功就没什么可退；回滚要写成幂等的
	}
	return payment.Refund(ctx, o.chargeID)
}

type notify struct{}

func (notify) Process(ctx context.Context, o *Order) error { return sms.Send(ctx, o.ID) }
func (notify) Rollback(context.Context, *Order) error      { return nil }
func (notify) Dependency() xflow.Dependency                { return xflow.Weak } // 弱依赖：失败只记一笔，继续往下走

var placeOrder = xflow.New[*Order]("place_order", reserveStock{}, charge{}, notify{})

func Place(ctx context.Context, o *Order) error {
	res := placeOrder.Execute(ctx, o)
	if len(res.RollbackErrors) > 0 { // 有资源没补偿回来，要人工介入
		slog.ErrorContext(ctx, "order compensation failed", "order_id", o.ID, "result", res.String())
	}
	return res.Err // 强依赖失败或 ctx 取消时非 nil
}
```

`charge` 失败时 `reserveStock` 被回滚、`notify` 不执行；`notify` 失败只记进 `res.Skipped`，订单照常成功。

## 单独使用：不跑 `xone.Run`

xflow 在核心 module 里（`go get github.com/xiaoshicae/x-one`，核心只依赖 yaml），上面的代码不调 `xone.Run` 也原样能跑。
区别只在配置：`XFlow` 块只在框架启动时读，不跑框架就是默认值（监控开着、回滚预算 30s）。要改就写在代码里：

```go
var placeOrder = xflow.New[*Order]("place_order", reserveStock{}, charge{}, notify{}).
	WithRollbackTimeout(10 * time.Second) // 只管这个流程，压过 XFlow.RollbackTimeout

func init() { xflow.SetMonitor(nil) } // 不要每一步的监控日志；换成自己的实现就传它
```

`WithRollbackTimeout` 返回一个新流程，原来那个不变；传 0 或负数直接 panic。跑框架时也能用，给个别流程单独定预算。

## 重点

- **回滚不沿用调用方的 ctx**：请求一超时，补偿最需要执行，那时原来的 ctx 已经取消了。回滚改由 `XFlow.RollbackTimeout`（默认 30s）限时，个别流程可以用 `WithRollbackTimeout` 单独定。
- **预算到点就不再等**：不看 ctx 的 `Rollback` 到点也会被放弃、记进 `RollbackErrors`，但它的协程仍在后台跑、仍可能读写 `data`。
- 弱依赖失败之后同样会被纳入回滚范围，所以 `Rollback` 不能假设 `Process` 成功过。
- xflow 不开 Span，要链路就在步骤里自己 `otel.Tracer(...).Start`。

## 配置

```yaml
XFlow:
  Monitor: true            # 关掉之后 Execute 一次监控回调都不走
  RollbackTimeout: 30s     # 回滚全部步骤的总预算，必须 > 0
```

- 回滚不沿用调用方的 ctx，否则请求一超时补偿必然全部失败——而补偿最需要执行的恰恰是那时候。
- 预算对不看 ctx 的 `Rollback` 同样有效：到点就不再等它，那一步记进 `RollbackErrors`，`Execute` 随即返回；
  被放弃的那一步的协程仍在后台跑、仍可能读写 `data`。

## 可观测

### 链路

| 来源 | Span 名 | 关键属性 |
|---|---|---|
| xflow | —— | xflow 不开 Span，步骤里自己用 `otel.Tracer(...)` |

链路的全貌、传播与信任边界见 [`docs/observability.md`「链路」](../docs/observability.md#链路)。
