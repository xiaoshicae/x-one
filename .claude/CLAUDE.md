# xone 项目约定

xone 是一个 Go 三方库集成框架：统一读配置、按阶段初始化、逆序关闭，使用者拿到的是**原生 client**
（`*gorm.DB`、`*redis.Client`、`*gin.Engine`）。**详细理由和表格见 `docs/development.md`**，这里只列要照做的。

## 语言

- 对话、代码注释、README 与 docs 用简体中文；测试函数名、基准函数名用中文（描述场景）。
- **error / panic 的消息、日志的 message 与字段名用英文**——它们会落进使用者的日志平台和告警。commit message 用英文。
- 文档和注释里的示例用中性的值（`Europe/Berlin`、`prod,eu`），不写尚未发布的「之前的版本 / 行为变化」。

## 写代码

- 不做过度优化：只有基准测试量过、且改完不比改前复杂才动手；引入缓存层、对象池、`unsafe` 先说明为什么没有别的办法。
- 模块对外返回的错误一律是 `*xerror.Error`：`xerror.Newf("xgorm", "connect", "cannot reach %s: %w", addr, err)`。
  - 底层错误用 `%w`，不用 `%v`；一个模块边界只包一次，内部的中间错误保持普通 error；消息里不重复模块名。
  - op 只从这组词里选：`config` `init` `new` `connect` `close` `register` `start` `stop` `execute`。
- 接一个库或升级一个库：写进文档的每一句行为描述先用代码量出来；没显式设的字段逐个确认它的默认值；
  数字连同依赖版本写进注释和 `docs/behavior.md`。
- 设计原则（全文见 `docs/architecture.md`）：
  - 给使用者的接口保持最简，复杂性沉到框架里——先问「使用者要多学什么」。
  - `init()` 只登记，不初始化；真正的初始化由框架按档位执行。
  - 每个带三方依赖的集成是独立的 Go module；零依赖的才留在核心。
  - 每个集成导出纯构造器 `New`：不碰全局、不读文件；会阻塞的才收 `ctx`。
  - 接入框架只有 `xhook.BeforeStart` / `BeforeStop`：业务不写档位（`StageBusiness`），客户端类集成写 `xhook.At(xhook.StageClient)`；
    停止钩子不写档位，它继承配对的启动钩子。集成只 import `xhook` / `xconfig`，不许 import 根包。
- 退出信号：信号在读配置之前接管；钩子收 `ctx`、框架在钩子之间复查；第一个信号之后还回默认处置。三件事缺一不可。

## 配置与文档

- 默认值预填在结构体里，未知字段是错误，`${VAR}` 未设置是错误，校验写在 `Validate()` 里。
- 新增或改动 Config 字段（含字段注释）：`go run ./internal/schemagen`，再写进 `docs/config.md` 自己那一节（`check.sh` 双向检查）。
- `config.md` 只放参考；实测数字放 `behavior.md`，日志 / 指标 / Span 名放 `observability.md`，新的报错文案放 `troubleshooting.md`。
- 删掉一个公开名字时，把它加进 `scripts/check.sh` 里的 `gone` 表。
- README 的第一个 ```go 代码块会被 `example/readme_test.go` 编译，改公开 API 时跟着改。
- 更新日志：打第一个版本 tag 之前不记录；之后使用者可见的变化在同一提交写进 `docs/CHANGELOG.md` 的「未发布」。

## 变异测试

- 改完安全或生命周期相关的代码跑一次 `scripts/mutate.py --only <module>`；活下来的变异 = 一条没有牙齿的承诺。
- 变异表在 `scripts/mutations/<module>.py`（根模块是 `core.py`），一行 `mutate(名字, 文件, 目录, 测试过滤, 改法...)`，
  改法只用 `swap()` / `cut()`，不裸写 `s.replace()`。
- 重构挪动了代码，对应的变异跟着挪（`--dry-run` 会报出对不上的模式）；变异打在调用点上，不只是被调用的函数里。

## 常用命令

```bash
scripts/test.sh                      # 全量测试（逐模块 GOWORK=off，带 -race）
scripts/check.sh                     # 架构约束 + 依赖边界 + 文档 + gofmt/vet（含 mutate.py --dry-run）
scripts/mutate.py [--only X] [-k X] [-j N] [--dry-run]   # 变异测试，经 overlay 跑，不要求干净工作区
scripts/e2e.sh [--load] [-run X]     # 真实服务测试：拉起 PG / MySQL / Redis / ClickHouse；CI 里是 .github/workflows/e2e.yml
docker compose -f e2e/compose.yml up -d --wait   # 本机没装这些服务时
scripts/release.sh vX.Y.Z --apply    # 打 tag（不推送）；--verify 推送之后验证装得上；--e2e-passed 跳过 e2e
go test -run=NONE -bench=. -benchtime=100000x ./xlog/ ./xflow/ ./xgin/middleware/   # 改热点代码前后各跑一次
```
