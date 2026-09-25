# 更新日志

这里记录每个版本里**使用者看得见的变化**：新功能、行为变化、不兼容变更、修复、性能。
仓库内部的重构和工具改动不写，除非它改变了使用者要做的事。

格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/)。所有模块共用同一个版本号（见 `scripts/release.sh`）。

## [未发布]

## [v0.1.0] - 2026-09-25

首个版本。所有模块同时发布、共用这个版本号：

- 核心 `github.com/xiaoshicae/x-one`：`xone.Run` / `MustRun`（按档位启动、逆序关闭、一份停止预算）、`xone.Func` / `xone.UntilSignal`、
  `xconfig`、`xhook`、`xerror`、`xlog`、`xapp`、`xflow`、`xtls`、`xonetest`。
- 集成：`xgin`、`xginswagger`、`xgorm`（MySQL / PostgreSQL）、`xgorm/clickhouse`、`xredis`、`xcache`、`xhttp`、`xtrace`、`xmetric`。
