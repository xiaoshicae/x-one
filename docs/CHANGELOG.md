# 更新日志

这里记录每个版本里**使用者看得见的变化**：新功能、行为变化、不兼容变更、修复、性能。
仓库内部的重构和工具改动不写，除非它改变了使用者要做的事。

格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/)。所有模块共用同一个版本号（见 `scripts/release.sh`）。

## [未发布]

### 不兼容变更

- 一级的 `App`、`Import`、`Profiles` 收进一个 `XApp` 块，`Profiles.Active` 去掉一层直接写值。旧写法启动失败，
  错误里说明怎么改。迁移：

  ```text
  # v0.1.0                      # 现在
  App:                          XApp:
    Name: order-api               Name: order-api
    Version: v1.2.0               Version: v1.2.0
  Profiles:                       Profiles: ${APP_ENV:dev}
    Active: ${APP_ENV:dev}        Import: [common/log.yml]
  Import: [common/log.yml]
  ```

  环境文件、被引入的文件里的 `Import` 同样挪到 `XApp.Import`。`xconfig.Unmarshal("App", …)` 改成 `"XApp"`。
- `xcache.Get` 改成泛型，按类型取值，不用再自己断言；存的类型对不上当作没命中（`XONE_DEBUG` 开着时会说明）。
  迁移：`v, ok := xcache.Get(k); u := v.(*User)` 改成 `u, ok := xcache.Get[*User](k)`；什么类型都收就写 `xcache.Get[any](k)`。

### 新增

- `XONE_DEBUG=1` 启动时往 stderr 打出用了哪个配置文件、激活了哪些 profile、按优先级读了哪些文件、合并之后的完整配置
  （凭证遮成 `***`），以及启动钩子的执行顺序。只在本地排查时开：输出是多行纯文本，不是 JSON。
- 启动 banner：只在 stderr 是终端时打，带 x-one 的版本；容器里、重定向或接在日志采集器后面时不写。
- `XApp` 块跟着框架一起来：只用核心、没 import xapp 的程序写了 `XApp.Name` 也能启动。
- `xlog.CtxWithKV(ctx, kvs)`：派生一个带着额外字段的 ctx，只影响用它写的日志；批量处理的每一条、起的每个 goroutine
  各带各的字段，不串到兄弟和父 ctx。`AddKV` 照旧是原地写、整个请求可见。

## [v0.1.0] - 2026-09-25

首个版本。所有模块同时发布、共用这个版本号：

- 核心 `github.com/xiaoshicae/x-one`：`xone.Run` / `MustRun`（按档位启动、逆序关闭、一份停止预算）、`xone.Func` / `xone.UntilSignal`、
  `xconfig`、`xhook`、`xerror`、`xlog`、`xapp`、`xflow`、`xtls`、`xonetest`。
- 集成：`xgin`、`xginswagger`、`xgorm`（MySQL / PostgreSQL）、`xgorm/clickhouse`、`xredis`、`xcache`、`xhttp`、`xtrace`、`xmetric`。
