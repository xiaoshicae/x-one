# xone 项目约定

xone 是一个 Go 三方库集成框架：统一读配置、按阶段初始化、逆序关闭，
使用者拿到的是**原生 client**（`*gorm.DB`、`*redis.Client`、`*gin.Engine`）。

## 语言

| 位置 | 语言 |
|---|---|
| 对话、代码注释、README 与 docs | 简体中文 |
| **error / panic 的消息** | **英文** |
| **日志的 message 与字段名** | **英文** |
| commit message | 英文 |
| 测试函数名、基准函数名 | 中文（描述场景，便于定位） |

这是一个给别人用的库。**它产出的错误和日志会落进使用者的系统里**——
进他们的告警、他们的日志检索、他们的 issue。中文字段名还会变成 JSON 的 key，
让日志平台的索引和看板直接对不上。注释是写给读这份代码的人的，那是另一回事。

```go
// 连接池满了就等着，不新建连接 —— 注释用中文
return fmt.Errorf("xgorm: connect to %s failed: %w", addr, err)   // 错误用英文
slog.Info("xgorm ready", "driver", info.Driver, "addr", info.Addr) // 日志用英文
```

## 优雅优先于快

**不做过度优化。** 代码要保持优雅、整洁、好读——这比省下几十纳秒重要得多。

只有同时满足以下两条才动手优化：

1. **量过**。有基准测试给出改前改后的数字，不接受"看着像是慢"。
2. **改完的代码不比改前复杂**。最好是更简单——把一次多余的分配去掉、
   把白干的活挪到条件后面，这类改动往往同时让代码更清楚。

为了性能引入缓存层、对象池、手写序列化、`unsafe`，一律先说明为什么
没有别的办法。已有的基准测试在 `*_test.go` 里，改热点代码前后各跑一次：

```bash
go test -run=NONE -bench=. -benchtime=100000x ./xlog/ ./xflow/ ./xgin/middleware/
```

## 错误一律用 xerror

模块对外返回的每一个错误都是 `*xerror.Error`，带上模块名和操作名：

```go
return xerror.Newf("xgorm", "connect", "cannot reach %s: %w", info.Addr, err)
return xerror.New("xgorm", "init", err)
```

调用方因此永远可以问「这是谁报的」：

```go
xerror.Is(err, "xconfig")   // 整条链里有没有 xconfig 的错误
xerror.Module(err)          // 最外层是谁报的
```

四条规矩：

1. **底层错误一律用 `%w`，不用 `%v`。** `%v` 把错误变成一段文本，
   `errors.Is` / `errors.As` 到此为止 —— 调用方再也判断不了根因是什么。
2. **一个模块边界一个 xerror，不是一层一个。** 内部的中间错误（比如
   `Config.validate()` 返回的那些）保持普通 error，由边界那一层包一次。
   每层都包的话文本会套成
   `xone xgin config failed, err=[xone xgin validate failed, err=[...]]`，
   信息没多，噪声翻倍。xerror 自己也兜着这一条：`New` 遇到同模块的 Error
   原样返回；`Newf` 的参数里有 Error 时渲染会折叠——同模块的只留「op: 原因」，
   别的模块的去掉重复的 `xone ` 前缀。兜底不是许可，边界上照样只包一次。
3. **消息里不再重复模块名。** 外框已经有了，再写一遍就是
   `xone xgorm init failed, err=[xgorm: ...]`。
4. **op 从这组词里选**，不要每处现编：

   | op | 用在 |
   |---|---|
   | `config` | 配置不合法、解码失败 |
   | `init` | 组件初始化（框架调的那次） |
   | `new` | 构造实例 |
   | `connect` | 建连、探测 |
   | `close` | 关闭、释放 |
   | `register` | 注册指标、注册方言 |
   | `start` / `stop` | 服务启停 |
   | `execute` | 跑一次业务流程（xflow） |

## 第三方库的默认值一律要量过

这个仓库被外部 review 挑出来的问题里，**大半是同一个毛病**：接了一个库，
用了它的默认行为，没量过它到底是什么行为，然后按自己以为的那个写进文档。

已经踩过的（每一条都是真的量出来才发现的）：

| 库 | 以为的 | 实际的 |
|---|---|---|
| gin | `TrustedProxies` 默认安全 | 默认 `0.0.0.0/0`，谁发 `X-Forwarded-For` 谁就是 `client_ip` |
| gin | `MaxMultipartMemory` 是请求体上限 | 是落盘阈值，堆开销约为它的三倍；32MB 默认 = 每请求 96MB |
| gorm | 不给 Logger 就是不打日志 | 补上它自己的默认：带 ANSI 颜色写 `os.Stdout` |
| gorm | `gorm.Open` 只装配 | 会自己 ping 一次，用的是它自己的 context |
| go-redis | 命令听调用方的 deadline | 默认不听，只认 `ReadTimeout`；实测 200ms 的预算等满 5s |
| ristretto | `MaxCost` 就是容量 | 每条另加 56 字节内部开销，配 2000 实际存 35 条 |
| resty | `Timeout` 管一次请求 | 管一次尝试；配 300ms + 3 次重试实测跑 1.24s |
| otelhttp | `CloseIdleConnections` 能传下去 | 它没实现，整条调用变成空操作 |
| net/http | `Shutdown` 超时会断开连接 | 只返回错误，在途连接照跑 |
| x/net h2c | `h2c.NewHandler` 的连接归 `Shutdown` 管 | 连接被劫持走，`Shutdown` 约 60µs 就返回 nil，在途请求照跑 |
| resty | 不给 logger 就不打日志 | 往 stderr 写 WARN / ERROR，URL 带着查询串 |
| resty | `resty.New()` 和 `NewWithClient` 一样 | 前者自带 cookie jar，不相干的调用之间串 cookie |
| otelhttp | 出站 Span 里没有凭证 | `url.full` 带着查询串，只去掉了 user:password |
| ristretto | `Close()` 能和在途读写并发 | 先关内部 channel、后置标记，实测并发读写里 750 个协程 panic |
| gorm | 日志里的 SQL 带的是占位符 | Logger 不实现 `ParamsFilter` 就代进真实参数值 |
| redisotel | Span 里只有命令名 | 默认 `db.statement` 带整条命令，`SET k v` 的值原样导出 |
| gorm ClickHouse 驱动 | 建连受调用方的 ctx 管 | `Initialize` 用 `context.Background()` 查版本：ctx 取消了也等满 dial_timeout，也不重试 |
| client_golang | 不合规的 Namespace 会报错 | 不报错，导出时转义：`my-app` 变成 `my_app_` |
| OTel SDK | `AlwaysSample` 尊重上游的采样决定 | 无视上游的 `sampled=00`，还把 `-01` 往下游传 |

所以接一个新库、或者升级一个库的时候：

1. **写进文档的每一句行为描述，先用一段代码量出来**，别照抄它的 README。
2. **我们没显式设的字段就是我们接受了它的默认值**——列一遍这些字段，
   逐个问「它的默认值是什么，我知道吗」。
3. 量出来的数字写进注释和 `docs/config.md`。后来的人不必再量一次，
   升级依赖之后数字对不上也能立刻看出来。

`scripts/mutate.py` 是这件事的兜底：把每条承诺对应的代码改坏，看有没有测试会失败。
活下来的变异 = 一条没有牙齿的承诺。改完安全或生命周期相关的代码跑一次。

**重构挪动了代码之后，对应的变异要跟着挪。** 变异模式失效（改不动任何东西）
和「改坏了没人发现」一样严重：那条承诺这一轮根本没被检查。脚本会把这种情况
单独报出来并且整轮失败——它自己就这样烂过两次，`applyConfig` 挪走
`SetTrustedProxies`、xerror 改了 `safeNew` 的签名，对应的变异都从此没再跑过。

所以变异是 `mutate.py` 表里的一行 `mutate(名字, 文件, 模块, 测试过滤, 改法...)`，
改法只写 `swap()` / `cut()`，它们自带「模式恰好匹配 N 处」的断言。
裸写 `s.replace()` 有两种烂法：模式不再匹配就
静默空转，模式匹配到多处就一次改坏两个地方——后者测试照样会红，
但红的已经不是你要验的那条承诺了。`scripts/mutate.py --dry-run` 只在内存里把
这些断言过一遍、不跑 go，不到一秒，`check.sh` 每次都跑：代码挪走了当场就红，
不用等到下次想起来跑全量。

**变异要打在调用点上，不只是被调用的函数里。** 「指标的 method 标签收敛」
原先只有一个直接调 `normalizeMethod` 的单元测试：函数本身是对的，
但没人验证中间件真的在用它。把调用点绕开（`normalizeMethod(m)` → `m`）
测试照过，而那正是这个 bug 的形状。

## 设计原则

**给使用者的接口保持最简，复杂性沉到框架里。** 这一条压过下面每一条：
能在框架内部消化的，就不推给使用者——不加新名词、不改他们写的签名、
不加一条「你得记住……」的规矩。评估一个改动时先问「使用者要多学什么」，
答案不是「什么都不用」就再想想。已经按这条做了的：配置什么时候读都行
（不用知道加载时机）、Runnable 只要写 Start、退出只配一个总时长、
C() 取不到时说清是调早了还是没配。

1. **`init()` 只登记，不初始化**——真正的初始化由框架按 Stage 档位执行，
   Go 的 init 顺序完全不影响结果。
2. **每个集成是独立的 Go module**——Go 的 MVS 会把整个模块图的版本要求
   强加给使用者，哪怕他一个包都没 import。零依赖的集成才留在核心里。
3. **每个集成必须导出纯构造器 `New`**——不碰全局、不读文件、不依赖框架。
   会阻塞的（建连、探测）多收一个 `ctx`，不会阻塞的不收：签名如实说明
   这个构造器会不会把你卡住。

4. **接入框架只有两个动词**——`xhook.BeforeStart(fn)` / `xhook.BeforeStop(fn)`，
   钩子里写普通的 Go 代码：`xconfig.Unmarshal(key, &c)` 读配置，自己建实例、
   自己存、自己写 `C()`。没有句柄、没有泛型、没有要先理解才能用的名词。
   档位用 `xhook.At(...)`，不写就是 `StageBusiness`（使用者的业务代码）；
   框架自带的客户端类集成（xgorm、xredis、xhttp、xcache）显式写 `StageClient`，
   只有必须早于或晚于别人的才写。同档内的顺序是 Go 初始化包的顺序，不能依赖。
   `Unmarshal` 什么时候调都行：第一次调用时才加载配置文件，读到的永远是最终值，
   不会因为读得早就静默给默认值。
   一起登记的就是一对：停止钩子只在同一个包里、在它之前最近登记的那个启动钩子
   成功之后才执行，所以停止钩子里不必处理「还没建起来」。

## 退出信号

注册信号处理会**取消系统默认的「收到就死」**，所以：信号在读配置之前就接管；
钩子收 `ctx` 且框架在钩子之间复查；第一个信号之后把默认处置
还回去，好让第二个信号能终止卡住的进程。三件事缺一不可，细节见 `docs/architecture.md`。

## 配置

默认值预填在结构体里，未知字段是错误，`${VAR}` 未设置是错误。
新增或改动 Config 字段必须同步 `docs/config.md`（`check.sh` 会检查）。

## 更新日志

使用者看得见的变化（新功能、行为变化、不兼容变更、修复、性能）**在同一个提交里**
写进 `docs/CHANGELOG.md` 的「未发布」一节，按「不兼容变更 / 新增 / 修复 / 性能」归类，
一条一句话，写使用者要知道什么、要改什么，不写实现细节。仓库内部的重构和工具改动不写。
不兼容变更必须写清旧写法怎么迁移。发版时把「未发布」改成 `[vX.Y.Z] - 日期`，
再在上面开一个新的空「未发布」。

## 常用命令

```bash
scripts/test.sh                      # 全量测试（跨 module，带 -race）
scripts/check.sh                     # 架构约束 + 依赖边界 + gofmt/vet
scripts/mutate.py                    # 变异测试：哪些承诺没有测试盯着（要干净工作区，几分钟）
scripts/e2e.sh [--load] [-run X]     # 真实 Web 服务测试：拉起 PG / Redis，起进程、发信号（不在 CI 里）；--load 连压测一起跑
scripts/release.sh vX.Y.Z --apply    # 打 tag（不推送）
scripts/release.sh vX.Y.Z --verify   # 推送之后：验证发出去的版本装得上、跑得起来
```
