# xlog —— 日志

基于 `log/slog` 的日志，有链路时自动带 `trace_id`（核心模块）。业务代码直接用标准库：

```go
slog.InfoContext(ctx, "order created", "order_id", id)
xlog.AddKV(ctx, "user_id", uid) // 之后同一请求里的每条日志都带着它
```

日志的全局约定（`trace_id` 注入、`xlog.AddKV`、框架的启停日志）见 [`docs/observability.md`](../docs/observability.md#日志)。

## 配置

装进标准库的 `slog.Default()`，业务代码直接用 `slog.InfoContext`。

```yaml
XLog:
  Level: info              # debug / info / warn / error，默认 info
  Format: json             # json / text，默认 json
  AddSource: false         # 记代码位置，有开销，默认关
  Timezone: ""             # 时间戳按哪个 IANA 时区渲染，如 Europe/Berlin；空 = 进程本地时区
  Console: true            # 打到标准输出，默认开
  File:
    Enable: false          # 默认关
    Path: /var/log/app     # 目录
    Name: app.log          # 实际文件带时间后缀，另有同名符号链接指向当前文件
    RotateTime: 24h        # 轮转周期，按本地时区对齐，默认一天，至少 1m
    MaxAge: 168h           # 历史保留时长，默认 7 天；0 = 不清理，负数启动失败
    Perm: "0644"           # 按八进制解析的字符串，0644 / 644 / 0o644 都认
```

- `Timezone` 配了却加载不到直接启动失败；scratch / distroless 镜像要 `import _ "time/tzdata"`。代码里用 `xlog.Location()`
  取生效中的时区：`t.In(xlog.Location()).Format(time.RFC3339)`。
- 文件名后缀随 `RotateTime` 的粒度：≥ 24h 是 `app.log.20260918`，≥ 1h 是 `app.log.2026091815`，更短是 `app.log.202609181504`。
- `Name` 那个位置上已经有一个普通文件（不是符号链接）时启动失败，不会把旧日志吞掉。
- 清理只删 `app.log.<时间后缀>` 这种自己命名的文件，启动时一次、之后每次轮转一次；`app.log.bak`、`app.log.1.gz` 不碰。
- 有链路时每条日志自动带 `trace_id` / `span_id`；请求级字段用 `xlog.AddKV(ctx, k, v)`，见 [observability.md](../docs/observability.md#日志)。

## 行为与实测

实测环境和跨模块的总表见 [`docs/behavior.md`](../docs/behavior.md)。

**`Perm` 为什么是字符串**：实测 yaml.v3 把 `0644` 解析成 420（对的），漏掉前导 0 写成 `644` 却是十进制 644 = 0o1204，
不报错。所以按八进制解析字符串，`0644`、`644`、`0o644` 都认。

**`RotateTime` 的文件名后缀**按周期取粒度：一天及以上是 `app.log.20260918`，一小时及以上是 `app.log.2026091815`，
更短是 `app.log.202609181504`。最细到分钟，所以短于 1m 直接启动失败：0s 实际每分钟一个文件，
30s 两个周期落在同一个文件名上。轮转按本地时区对齐。

**`Timezone` 配了却加载不到直接启动失败**，不会悄悄退回本地时区。scratch / distroless 镜像里没有
`/usr/share/zoneinfo`，要在自己的 `main` 包加一行 `import _ "time/tzdata"`（约 400KB）；框架不替你编，
没用到这项的人不该背这 400KB。
