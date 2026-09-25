# xflow —— 流程编排

流程编排，失败自动回滚（核心模块）。

```go
err := xflow.New[T](name, steps...).Execute(ctx, data)
```

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
