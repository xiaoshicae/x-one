# 参与开发

给改这个仓库的人看的：目录怎么分、每个脚本干什么、怎么写一个新集成、e2e 怎么跑。
项目约定（语言、错误、默认值要量过、变异测试）写在 [`.claude/CLAUDE.md`](../.claude/CLAUDE.md)，
设计上的来龙去脉在 [`architecture.md`](architecture.md)。

- [仓库结构](#仓库结构)
- [go.work 与 GOWORK=off](#gowork-与-goworkoff)
- [脚本](#脚本)
- [写一个自己的集成](#写一个自己的集成)
- [e2e：真实 Web 服务测试](#e2e真实-web-服务测试)

## 仓库结构

```
xone/
├── xone.go              根包：Run / MustRun / Option
├── xhook/               两个生命周期钩子，零依赖。集成包唯一需要认识的东西
├── internal/
│   ├── config/          配置加载：定位、profile / import、合并、占位符
│   ├── hook/            钩子登记与按档位执行
│   ├── xclient/         xgorm / xredis / xcache 共用的具名实例管理
│   └── schemagen/       生成 config_schema.json、核对 docs/config.md（独立的工具 module）
├── xerror/  xutil/      零第三方依赖
├── xapp/                应用身份（名字、版本），只认领配置不初始化
├── xconfig/             配置解码辅助：严格解码、单/多实例分派
├── xlog/                日志，基于 log/slog，零第三方依赖
├── xtrace/              链路，基于 OpenTelemetry（独立 module）
├── xmetric/             指标，基于 Prometheus（独立 module）
├── xgorm/               数据库，基于 GORM（独立 module）
│   └── clickhouse/      ClickHouse 驱动（独立 module）
├── xredis/              Redis，基于 go-redis（独立 module）
├── xcache/              本地缓存，基于 ristretto（独立 module）
├── xhttp/               出站 HTTP，基于 resty（独立 module）
├── xgin/                Web 服务，基于 Gin，内置四个中间件（独立 module）
├── xginswagger/         Swagger UI（独立 module，UI 资源不进普通服务）
├── xflow/               流程编排 + 自动回滚，零第三方依赖
├── xonetest/            给使用者的测试辅助：换一份配置、跑一遍钩子
├── docs/
│   ├── config.md        全部配置项参考
│   ├── architecture.md  设计原则与启停流程
│   ├── development.md   本文件
│   └── CHANGELOG.md     使用者看得见的变化
├── example/             可直接跑的示例，同时是进程内的跨模块集成测试
│   ├── consumer/        消息队列消费者：非 Web 服务的形状
│   └── component/       自己写一个集成：两个钩子 + 一个 C() 的完整样例
├── e2e/                 真实 Web 服务测试：真的进程、真的 PG / MySQL / Redis / ClickHouse、真的信号（独立 module，不发布）
│   ├── service/         被测服务：用齐各集成，配置全部来自 YAML
│   ├── baseline/        裸 gin 的对照服务，压测时比出框架的开销
│   ├── harness/         起进程、读日志 / Span / 指标 / /proc、TCP 故障代理、下游桩、压测器
│   └── compose.yml      e2e 要的 PG / MySQL / Redis / ClickHouse，本机没装时用
├── config_schema.json   配置的 JSON Schema，由结构体生成，给 IDE 用
├── .github/workflows/   ci.yml：check.sh + test.sh，外加用 Go 1.22 单独编译一遍核心；e2e.yml：e2e + 全量变异
└── scripts/
    ├── check.sh         把设计约束编译成检查
    ├── test.sh          跑全仓库测试（go test ./... 不跨模块边界）
    ├── mutate.py        变异测试：把每条承诺改坏，看有没有测试会失败
    ├── mutations/       变异表，一个 module 一个文件（core.py 是根模块）
    ├── e2e.sh           拉起 PG / MySQL / Redis / ClickHouse，跑 e2e/ 的真实 Web 服务测试
    └── release.sh       打 tag 发布，推送之后 --verify 验证装得上
```

脚本放在哪个目录下调都行，它们会先切到仓库根。

## go.work 与 GOWORK=off

仓库里提交了 `go.work`，把全部模块（核心、各集成、`example`、`e2e`、`internal/schemagen`）
放进一个工作区，模块之间另外靠各自 `go.mod` 里的 `replace` 互指。所以：

- IDE 打开根目录就认得全部模块；
- 在根目录 `go run ./example --config=example/application.yml` 也能跑。

但 `scripts/check.sh` 和 `scripts/test.sh` 一律用 `GOWORK=off` 逐模块跑——工作区会遮住
某个模块自己 `go.mod` 的问题（漏了 require、版本不对），那必须由 CI 抓出来。
同理，`go list -m all` 在工作区里会把所有模块的依赖并在一起，检查核心依赖足迹时必须 `GOWORK=off`。

## 脚本

| 命令 | 干什么 | 在 CI 里 |
|---|---|---|
| `scripts/check.sh` | 架构约束 + 依赖边界 + 文档 + gofmt / vet | 是 |
| `scripts/test.sh [go test 参数]` | 逐模块 `GOWORK=off go test -race ./...`，一个模块红了也跑完其余的，最后一起报 | 是（`-count=1`） |
| `scripts/mutate.py [-j N] [--only X] [-k X]` | 变异测试，并行跑，不动工作区 | 每晚（`--dry-run` 在 check.sh 里） |
| `scripts/e2e.sh [--load] [-run X]` | 真实 Web 服务测试，要 PG / MySQL / Redis（ClickHouse 可选） | 改了 go.mod / go.sum 的 PR、每晚（不含压测） |
| `scripts/release.sh vX.Y.Z [--apply \| --verify]` | 打 tag / 验证发布 | 否 |

### check.sh

把设计约束编译成检查，在 CI 里跑：

- 核心模块图不超过 3 个模块，核心不依赖任何集成模块；
- `xhook` / `xerror` / `xutil` 零第三方依赖；
- `init()` 只出现在集成包里；
- 公开 API 数量上限：根包 15、`xhook` 6、`xconfig` 6；
- 集成包必须导出 `New`，且不许 import 根包；
- **每个配置字段都写进了 `docs/config.md` 自己那一节**，每一节的 YAML 示例都过得了 schema；
- `config_schema.json` 与结构体一致；
- 错误和日志等运行期字符串是英文；错误走 `xerror`、用 `%w` 包底层错误；
- `*.go` / `*.md` / `*.yml` 里不再出现已经删掉的公开名字；
- `scripts/mutate.py --dry-run` 的每条变异模式都还对得上代码；
- gofmt、每个模块的 `go vet`。

### config_schema.json

由 `go run ./internal/schemagen` 从各模块的 Config 结构体生成，字段说明直接取结构体上的
注释——注释、文档、schema 是同一个来源。改了 Config 字段之后：

```bash
go run ./internal/schemagen      # 重新生成
```

再把字段写进 `docs/config.md` 对应的那一节。忘了任何一步 `check.sh` 都会红。

### mutate.py

全量每晚在 CI 里跑一次（`e2e.yml` 的 `mutate`），改完安全或生命周期相关的代码之后也手动跑一次。
它把每条承诺对应的代码改坏，看有没有测试会失败——活下来的变异 = 一条没有牙齿的承诺：
代码写着、文档写着，改坏了却没人知道。

```bash
scripts/mutate.py                  # 全量：改坏、编译、跑测试，默认开 CPU 数那么多路
scripts/mutate.py -j 2             # 只开两路
scripts/mutate.py --only xgorm     # 只跑一个表 / module / 路径前缀（xgorm 连带 xgorm/clickhouse；xlog/ 只跑改 xlog 的）
scripts/mutate.py -k 超时          # 只跑名字里带这段的
scripts/mutate.py --dry-run        # 只查每条变异的模式还对不对得上代码，不到一秒（过滤照样生效）
```

改坏的副本放在临时目录，经 `go test -overlay` 换进编译，工作区一个字节都不动：
不要求干净的工作区，跑的时候照样可以改代码，Ctrl-C 了也没有要写回的文件。
结果按表里的顺序打印，不按完成的先后，两轮的输出可以直接 diff。

变异表在 `scripts/mutations/` 下，一个 Go module 一个文件：`core.py` 是根模块（含 `./xhook` 这类
根模块里的目录），其余按 module 目录名，如 `xgin.py`、`xgorm.py`、`clickhouse.py`、`schemagen.py`。
一行放在哪个文件，看它在哪个目录下跑测试（第三个参数），放错了 `mutate.py` 直接报错。
每条变异是一行 `mutate(名字, 文件, 目录, 测试过滤, 改法...)`，改法只用 `swap()` / `cut()`，
它们自带「模式恰好匹配 N 处」的断言。重构挪动了代码，对应的变异要跟着挪；
变异要打在调用点上，不只是被调用的函数里。规矩的全文见 `.claude/CLAUDE.md`。

### release.sh

多模块仓库每个 module 有自己的 tag，发布前要把开发用的 `replace` 换成真实版本号。

```bash
scripts/release.sh v0.1.0            # 跑检查、测试和 e2e，再打印要做什么，不改任何东西
scripts/release.sh v0.1.0 --apply    # 改 go.mod、提交、打 tag，再提交一次把 replace 还原（不推送）
git push origin main --tags          # 推送由人来做：module proxy 永久缓存 tag，推错了删不掉
scripts/release.sh v0.1.0 --verify   # 推送之后：在一个全新的外部工程里 go get，验证装得上、跑得起来
```

发布前要一轮绿的 e2e：第 2 步会跑 `scripts/e2e.sh`（要 PG / MySQL / Redis）。这个提交刚在别处跑绿过
（比如手动触发了一次 CI 的 e2e 工作流）的话，加 `--e2e-passed` 跳过，由你担保。
检查、测试、e2e 的输出都照常打印，红了直接看得到是哪一行。

各子模块 `go.mod` 里仓库内的 require 一律钉成要发的版本，不管原来写的是 `v0.0.0`
还是 go 工具补的伪版本（`xgin` 里 `xmetric` 的 `v0.0.0-2026…`）；仓库内的 replace 全部去掉，
只出现在 replace 里、没被 require 的不会被补成 require。

只收 v0 / v1（模块路径没有 `/vN` 后缀）。`example`、`e2e`、`internal/schemagen` 不发布。

### 基准测试

改热点代码前后各跑一次，改动要有数字支撑（见 `.claude/CLAUDE.md`「优雅优先于快」）：

```bash
go test -run=NONE -bench=. -benchtime=100000x ./xlog/ ./xflow/ ./xgin/middleware/
```

## 写一个自己的集成

一个集成就是一个包：纯构造器 `New`，加上两个钩子把它接进框架，再给使用者一个 `C()`。

```go
package xmine

const ConfigKey = "XMine"

type Config struct {
	Addr string `yaml:"Addr"` // 一定要写 yaml tag
}

func DefaultConfig() Config { return Config{Addr: "127.0.0.1:1234"} }

// New 纯构造：不碰全局、不读文件、不依赖框架。会阻塞的才收 ctx
func New(ctx context.Context, c Config) (*Client, io.Closer, error) { /* ... */ }

func init() {
	xhook.BeforeStart(initXMine, xhook.At(xhook.StageClient))
	xhook.BeforeStop(closeXMine) // 档位跟着上面那个启动钩子
}

func initXMine(ctx context.Context) error {
	c := DefaultConfig()
	if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
		return err
	}
	client, closer, err := New(ctx, c)
	if err != nil {
		return err
	}
	setDefault(client, closer) // 存到哪、怎么取，是你自己的事
	return nil
}

func closeXMine(context.Context) error { return currentCloser().Close() }

// C 给使用者的入口：返回原生类型
func C() *Client { /* ... */ }
```

要点：

- 只 import `xhook` / `xconfig`，**不许 import 根包**（`check.sh` 会查）。
- 客户端类的写 `StageClient`；业务资源不写档位（默认 `StageBusiness`）。
- 停止钩子只在它前面那个启动钩子成功之后才执行，里面不必处理「还没建起来」。
- 错误用 `xerror.New` / `xerror.Newf` 包一次，底层错误用 `%w`；错误和日志消息用英文。
- 带三方依赖的集成做成独立的 Go module，别让它的依赖进核心。
- 测钩子用 `xonetest`：`UseConfigYAML(t, yml)` 换一份配置，`StartHooks(t)` 按档位跑启动钩子、
  测试结束时跑配对的停止钩子。`example/` 不许 import `internal/`（`check.sh` 会查）——使用者 import 不到。

可运行的完整样例：[`example/component/xkv/`](../example/component/xkv/)。
多实例写法（`xconfig.UnmarshalClients`、按名字的 `C(name...)` / `Has` / `Names`）和
更多细节见 [`config.md`「写一个自己的集成」](config.md#写一个自己的集成)。

## e2e：真实 Web 服务测试

`e2e/` 把 `e2e/service` 编成二进制、当成一个真的进程起起来，连真的 PostgreSQL、MySQL、Redis 和 ClickHouse，
发请求、发信号，再读它的日志、Span、`/metrics` 和 `/proc`。

- 服务的 XGorm 是多实例：`default` 连 PostgreSQL，`mysql` 连 MySQL（`/mysql/...` 那组接口用 `xgorm.C("mysql")`）；
  第三个 `ch` 连 ClickHouse（`/ch/...`，经 `xgorm/clickhouse`），写在 profile `e2e/service/application-ch.yml` 里，
  只有 `TestClickHouse_*` 起的进程激活它（`harness.Options.ClickHouse`）；
- 故障经 harness 里的 TCP 代理注入（断开、拒绝新连接、加延迟、模拟主机宕机），不去停真的 PG / MySQL / Redis；
- 用例之间各用各的端口、表名和 key 前缀；
- PG / MySQL / Redis 没在跑时脚本会先按本机的装法拉起来（`pg_ctlcluster` / `service mysql` / `redis-server`），
  连接参数可以用 `XONE_E2E_PG_ADDR`、`XONE_E2E_MYSQL_ADDR`、`XONE_E2E_REDIS_ADDR` 等环境变量覆盖；
  服务归别处管（CI 的服务容器、docker compose、另一台机器）时设 `XONE_E2E_EXTERNAL=1`，或者地址本来就不在本机，
  脚本就只等它就绪（最多 60 秒），不去启动。就绪只看 TCP 连不连得上，不要求装 `psql` / `mysqladmin` / `redis-cli`；
- ClickHouse 跑在 Docker 容器 `xone-ch` 里（`XONE_E2E_CH_CONTAINER`；native 127.0.0.1:9000、HTTP 127.0.0.1:8123，`XONE_E2E_CH_*` 覆盖），
  停着就 `docker start`；没有 Docker、没有容器或起不来时脚本打一行提示并设 `XONE_E2E_CH=0`，`TestClickHouse_*` 各自跳过，其余照跑；
- 后面的参数原样交给 `go test`。

本机没装这些服务的话，用 `e2e/compose.yml` 一次起齐，账号密码和脚本的默认值一致：

```bash
docker compose -f e2e/compose.yml up -d --wait   # 起来并等到健康；默认端口被占了用 XONE_E2E_PG_PORT 等换一个
scripts/e2e.sh
docker compose -f e2e/compose.yml down           # 数据在 tmpfs 里，down 了就没了
```

```bash
scripts/e2e.sh                   # 全部（压测除外）
scripts/e2e.sh -run Smoke        # 只跑冒烟
scripts/e2e.sh --load -run Load  # 压测：裸 gin 对照三种中间件配置、并发 1～256，外加几分钟的浸泡
scripts/e2e.sh --load -run MySQL # MySQL 那一组，连同裸 gin + MySQL 对照 xone + MySQL 的压测
scripts/e2e.sh --load -run ClickHouse # ClickHouse 那一组，连同裸 gin + ClickHouse 对照 xone + ClickHouse 的压测
```

压测默认不跑（一轮十来分钟，要独占机器），`--load` 才跑。结果只在测试日志里打成表格：
QPS、p50 / p90 / p99 / max、错误数、服务进程每请求的 CPU 微秒和峰值 RSS，以及相对裸 gin 多出来的部分。
压测器、被测服务和 PG 在同一台机器上抢 CPU，这些数字只能拿来互相比较。

`test.sh` 遍历到 `e2e` 模块时，每个测试都因为 `XONE_E2E` 不为 1 而跳过，
没有数据库的机器上照样全绿。CI 里它是单独的工作流 `.github/workflows/e2e.yml`：改了任何 `go.mod` / `go.sum`
的 PR、每晚、手动触发时跑（不含压测），四个服务是服务容器；PG / MySQL / Redis 的 TLS 用例要以 root
在本机另起实例，runner 上跳过。任务摘要里写着 `KNOWN BUG` 跳过了几条。用例照文档写的行为断言，揭示了框架的 bug 时不改断言去迁就它，
而是标成 `KNOWN BUG` 跳过，证据写在用例的注释里。
