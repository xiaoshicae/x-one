# 架构与设计

这份文档讲 xone **为什么是现在这个样子**：设计原则、启停流程的每条保证、以及那些
量过之后才定下的默认值。只想用起来的话，[README](../README.md) 就够了；全部配置项见
[`config.md`](config.md)，参与开发见 [`development.md`](development.md)。

- [三条设计原则](#三条设计原则)
  - [一、`init()` 只登记，不初始化](#一init-只登记不初始化)
  - [二、每个集成是独立的 Go module](#二每个集成是独立的-go-module)
  - [三、每个集成必须导出纯构造器](#三每个集成必须导出纯构造器)
- [接入框架：两个钩子](#接入框架两个钩子)
  - [档位：只在你必须早于或晚于别人时才写](#档位只在你必须早于或晚于别人时才写)
  - [启动失败时不会调你的停止钩子](#启动失败时不会调你的停止钩子)
- [不是 Web 服务怎么办：consumer / job](#不是-web-服务怎么办consumer--job)
- [退出信号：从进程起步的第一毫秒就接管](#退出信号从进程起步的第一毫秒就接管)
- [停止预算是一份，不是每个组件一份](#停止预算是一份不是每个组件一份)
- [三个模块共用的那一份](#三个模块共用的那一份)
- [模块之间怎么互相扩展](#模块之间怎么互相扩展)
- [配置加载的几个决定](#配置加载的几个决定)
- [我们故意跟底层库不一样的地方](#我们故意跟底层库不一样的地方)
- [状态](#状态)

整体流程：

```
启动：接管信号 → 加载配置 → StageLog → StageTelemetry → StageClient → StageBusiness → StageServer → Runnable.Start
退出：取消 Start 的 ctx → Runnable.Stop（可选）→ 等 Start 返回 → 停止钩子逆序（StageServer → … → StageLog）
```

---

## 三条设计原则

### 一、`init()` 只登记，不初始化

Go 的 `init()` 执行顺序是「拓扑序 + 包路径字典序」，使用者控制不了。所以问题从来不是
`import _` 本身，而是**很多框架把「登记」和「初始化」合成了一件事**，于是那个不可控的
顺序就变成了初始化顺序。

xone 把它们拆开：`init()` 只把「我是谁、我要哪段配置、怎么初始化我」记进一个列表，
真正的初始化由框架在 `Run()` 里按 **Stage 档位**执行。**Go 的 init 顺序完全不影响结果。**

```
StageLog → StageTelemetry → StageClient → StageBusiness → StageServer
```

业务钩子不写档位，默认落在 `StageBusiness`：全部客户端都已就绪，钩子里直接
`xgorm.C()`；服务还没起来，预热、订阅做完了才接流量。关闭是严格逆序。
档位的细节见[下文](#档位只在你必须早于或晚于别人时才写)。

### 二、每个集成是独立的 Go module

这一条直接关系到使用体验。Go 的 MVS 会把**整个模块图**里的版本要求强加给使用者——
哪怕他一个包都没 import。实测过：一个只 import 了某个零依赖
错误包的应用，自己写死 `gin v1.9.1`，最终被顶到了 `v1.12.0`，还被定死了 gorm、redis、otel 的版本。

所以（数字是只 import 该集成时使用者的模块图大小）：

```
github.com/xiaoshicae/x-one           核心，2 个模块，Go 1.22
├── xapp  xlog  xconfig  xflow       零依赖，所以留在核心里
├── xtrace                           独立 module，23 个模块
├── xmetric                          独立 module，35 个模块
├── xcache                           独立 module，11 个模块
├── xhttp                            独立 module，54 个模块
├── xredis                           独立 module，55 个模块
├── xgorm                            独立 module，62 个模块
│   └── clickhouse                   独立 module，ClickHouse 驱动（+81 个模块）
├── xgin                             独立 module，83 个模块
└── xginswagger                      独立 module，82 个模块
```

**你不用的集成，它的依赖不会进你的模块图**，它要求的 Go 版本也不会。
除核心外的集成都因为上游而需要 Go 1.25，核心留在 1.22。
每个集成也能独立升大版本，不会因为某一个要改 API 就逼着整个框架升级。

零依赖的集成（`xlog`、`xflow`、`xapp`）留在核心模块里：分模块是为了把依赖挡在使用者之外，
没有依赖可挡就不必多一个模块。

同一条线也用在模块内部：

- **Swagger UI** 要把整套前端资源编进二进制，比只用 gin 多 26 个模块，所以它是
  `xginswagger` 而不是 `xgin` 的一部分——文档是开发期的事，不该让每个线上服务都背着。
- **数据库驱动**：一个只 import `xgorm` 的应用模块图是 65 个，加上 ClickHouse 驱动变成
  **128** 个（`go list -deps` 里的非标准库包 127 → 171；clickhouse-go v2.48.0）。多出来的大头是 Docker 和 testcontainers —— `clickhouse-go`
  用它们跑集成测试，而 `go.mod` 分不出「只测试用」，于是它们落在主 require 块里一路传给
  每个使用者。所以 mysql 和 postgres 内置，其余驱动由独立 module 提供，`xgorm` 只留一个注册点：

  ```go
  import (
  	"github.com/xiaoshicae/x-one/xgorm"
  	_ "github.com/xiaoshicae/x-one/xgorm/clickhouse"   // 用到才付钱
  )
  ```

- **测试依赖也算数**：`go mod tidy` 会把它记成直接依赖，一样进使用者的模块图。
  实测给 `xtrace` 加一个只用于测试的 `otelhttp`，使用者的模块图从 23 涨到 27。

### 三、每个集成必须导出纯构造器

```go
func New(cfg Config) (*T, io.Closer, error)                       // 不碰全局、不读文件、不依赖框架
func New(ctx context.Context, cfg Config) (*T, io.Closer, error)  // 会建连或探测的，多收一个 ctx
```

零装配是默认路径，`New()` 保证它**不是唯一路径**：测试直接调它拿一个干净实例（不需要 mock），
需要两套配置时也有出路。

会阻塞的构造器（`xgorm`、`xredis`、`xtrace`）收 `ctx`，不会阻塞的（`xlog`、`xmetric`、
`xcache`、`xhttp`）不收——签名如实说明这个构造器会不会把你卡住。

这一条写进了 CI 检查（`scripts/check.sh`）。

---

## 接入框架：两个钩子

接入框架只有两个动词：**在启动前跑这个，在停止前跑那个**。

```go
func init() {
	// 被业务依赖的客户端声明 StageClient；业务代码自己的钩子不写档位
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
```

钩子里写的是普通的 Go 代码：读配置、建实例、存起来。框架只负责在对的时候调它们，
以及按相反的顺序调停止钩子。**没有句柄、没有泛型、没有要先理解才能用的名词。**

`xconfig.Unmarshal` 把「默认值 + 配置文件覆盖」的结果填进你的结构体，认不出的字段
是错误，结构体实现了 `Validate() error` 的话解完会调一次。**在 `Start` 之前任何时候调都行**——
在 `main` 里、在 `xone.Run` 之前都一样：第一次调用时框架才去找配置文件、加载它，
读到的永远是最终值，不会因为读得早就静默拿到一份默认值。

可以跑的完整样例在 [`example/component/`](../example/component/)：一个自己写的
集成（`xkv/`）、一个业务配置块（`conf/`）、和一个用到它们的服务。写法的逐步说明见
[`config.md`「写一个自己的集成」](config.md#写一个自己的集成)。

### 档位：只在你必须早于或晚于别人时才写

```go
xhook.BeforeStart(initXLog, xhook.At(xhook.StageLog))
```

五档，含义写在名字里：

| 档位 | 用在 |
|---|---|
| `StageLog` | 日志：最先起、最后关，其余组件的启停日志才写得出去 |
| `StageTelemetry` | 链路与指标：要早于客户端，客户端的 Span 才挂得上、指标才收得到 |
| `StageClient` | 数据库、缓存、HTTP 客户端：被业务依赖。`xgorm` / `xredis` / `xhttp` / `xcache` 都在这一档 |
| `StageBusiness` | 你自己的业务资源（预热、定时任务、消费者）。**不写就是它** |
| `StageServer` | 对外服务：最后起、最先关 |

停止钩子不写档位时跟着和它配对的那个启动钩子（显式写了 `At` 的以它为准），停止顺序
又是启动的整体镜像，所以一个资源只在启动钩子上声明一次档位就同时管住了两头。

同一档内按登记顺序执行，也就是 Go 初始化各个包的顺序：同一份代码每次都一样，但它由
import 关系和包路径的字典序决定，不是 import 语句的书写顺序。所以有先后要求的东西
要放进不同的档位，别指望同档内的顺序。

用固定几档而不是任意数字，是因为数字需要全局协调：谁该填 30、谁该填 50，声明得越多
越没人说得清。档位不需要——同一档内的东西本来就互不依赖。

### 启动失败时不会调你的停止钩子

**一起登记的就是一对**：停止钩子只在同一个包里、在它之前最近登记的那个启动钩子
成功了之后才执行。所以 `closeXMine` 里不必处理「资源还没建起来」——那种情况下
它根本不会被调到。

之前没有启动钩子的停止钩子不依赖启动，总会执行——Runnable 写错、配置读不出来这类
一个启动钩子都没跑就失败的情况也一样（`WithStopTimeout` 配成非正数除外：那时没有可用的
停止预算，`Run` 什么都不做就返回）。

一个包要管好几样资源时，就一样一对地登记：

```go
xhook.BeforeStart(openA)
xhook.BeforeStop(closeA)
xhook.BeforeStart(openB)
xhook.BeforeStop(closeB)
```

`openB` 失败时 `closeA` 照常执行、`closeB` 不执行，各关各的。「哪个包」按登记时
调用栈上的那个 `init` 认，所以经一个辅助包替你登记的钩子也算在你的包头上。

---

## 不是 Web 服务怎么办：consumer / job

`xgin` 没有任何特殊地位，它只是众多 `Runnable` 实现中的一个。触发初始化的是
`xone.Run`，跟你用不用 xgin 无关：

```
loadConfig → StageLog → StageTelemetry → StageClient(XGorm/XRedis/XCache) → StageBusiness → 启动 Runnable
```

所以消费者服务要做的只有一件事：写一个 `Start`。

```go
func (c *Consumer) Start(ctx context.Context) error {
    for range c.workers {
        c.wg.Add(1)
        go func() { defer c.wg.Done(); c.loop(ctx) }()  // ctx 取消 → 不再取新消息
    }
    c.wg.Wait()          // 等在途消息做完，理由见下
    return c.q.Close()   // 在途消息做完之后才 Close，理由也见下
}

func main() { xone.MustRun(&Consumer{q: client, handle: handle}) }
```

`Stop` 不用写：`ctx` 被取消时 `Start` 自己返回就够了。光靠 `ctx` 停不下来的
（比如 `http.Server` 要调 `Shutdown`）再加一个 `Stop(ctx context.Context) error`，
框架退出时先调它、再等 `Start` 返回。签名写错（比如少了 `ctx`，或者 `Stop` 写在指针上、
传进来的却是值）会让 `Run` 直接报错——否则那个 `Stop` 永远不会被调到，服务收到信号也停不下来。

一次性任务更简单：`Start` 干完 `return nil`，框架立刻走正常的逆序关闭。

**三个容易踩的地方**，[`example/consumer/`](../example/consumer/) 里每一条都有测试钉着：

| | 为什么 |
|---|---|
| `Start` 必须等在途消息做完再返回 | 框架是在 `Start` 返回**之后**才关数据库和缓存的。提前返回，还在处理的消息就会摸到已经关掉的连接池 |
| 处理消息用的 ctx 要 `context.WithoutCancel` | 沿用已取消的 ctx，这条消息里每一次写库、每一次调下游、连最后那次 `Ack` 都会一进去就被拒绝——消息没做完，队列也没收到确认 |
| 在途消息做完之前别 `Close` 客户端 | 多数客户端 `Close` 时会顺带提交 offset，提前提交 = 退出时静默丢消息。写了 `Stop` 的也一样别在里面关：框架是先调 `Stop`、再等 `Start` 返回的，此刻 worker 还在处理在途消息 |

第二条和 `xflow` 的回滚是同一个道理：**补偿逻辑最需要跑完的时机，恰恰是退出的那一刻。**

单条消息的处理超时必须小于服务那一段停止预算（`xone.WithStopTimeout` 的 2/3，默认 10s）——
否则框架等不到就往下关资源了。xgin 等在途请求用的也是这一段。

业务自己的配置块怎么接进来，见 [`config.md`「业务自己的配置块」](config.md#业务自己的配置块)。

---

## 退出信号：从进程起步的第一毫秒就接管

注册信号处理这件事本身，**取消了系统原本的「收到就死」**。这是个开关，不是
「多加一个处理器」。所以它装在哪、什么时候还回去，直接决定进程杀不杀得掉。

xone 接管的是 SIGINT 和 SIGTERM，做法是三件事一起：

1. **信号在读配置之前就接管。** 装在初始化之后的话，连库、连 Redis、Ping 重试
   那几秒里 SIGTERM 走的是系统默认处置——进程当场暴毙，已经建好的资源一个都
   来不及注销（注册中心里那条记录、那把分布式锁，只能等对端超时过期）。
2. **钩子收 `ctx`，框架在每个钩子之前再查一次。** 收到信号后不再
   跑剩下的启动钩子，已建好的逆序关干净，服务不再启动，然后以 `0` 退出——
   按要求退出不是故障，报成失败的话每次滚动更新撞上这个窗口都会留一条「启动失败」。
3. **第一个信号之后把默认处置还回去。** 框架仍可能卡在某个不看 `ctx` 的第三方调用里，
   这时**再发一次信号进程立即终止**。没有这条逃生口，开关一打开就意味着卡住 =
   只有 `kill -9` 收得掉，K8s 得等满整个终止宽限期。

第 3 条同样覆盖关闭阶段：`Stop` 卡住时，第二次 Ctrl+C 有用。

## 停止预算是一份，不是每个组件一份

`xone.WithStopTimeout`（默认 15s）是**整个退出流程**的总预算：从开始退出到 `Run`
返回，最坏也就这么久，部署环境的终止宽限期留得比它长就够了。它必须真的管住
每一步，否则就只是一句话：

- 框架在内部把它分开用：服务（`Stop` + 等 `Start` 返回）最多占前 2/3，其余留给
  停止钩子，钩子之间再给排在后面的各留一份（`min(1s, 剩余时间 ÷ 钩子数)`）——
  不肯退出的服务、关不掉的连接池都吃不掉别人那份，排在最后的 xlog 总还有时间关文件。
- xgin 没有自己的停止超时：它的 `Stop` 就按服务那一段（总预算的 2/3，默认 10s）等在途请求，
  到点还没做完的强制断开，再等 handler 真正返回；还有没返回的，错误里写明几个。
  不看 `ctx` 的 handler 框架停不下来（Go 没有从外面终止协程的办法），慢操作要传
  `c.Request.Context()`。要让它等得更短或更长，调的是总预算——不存在第二个要和它对齐的数。
- 组件的 `Close()` 没有 `ctx` 可传，所以框架另起协程去等，到点就不再等它。
  一个连接池关不掉，不该让后面每个组件、以及进程本身都排在它后面。
- `WithStopTimeout` 必须为正：0 不是「不限时」而是「一点都不等」，`Run` 在做任何事之前就报错。

---

## 三个模块共用的那一份

`xgorm` / `xredis` / `xcache` 是同一个形状：配置里可以写一个实例也可以按名字写
好几个，运行时 `C()` 按名字取，启动时挨个建、有一个建不起来就把已建好的全关掉。

这些语义本该处处一致——「名字找不到时说什么」「两种写法混用怎么办」都是一次决定。
分散在三个模块里就是三份会各自漂移的实现，所以收进了核心：

| 共用的东西 | 在哪 |
|---|---|
| 具名实例、建实例、失败回滚、逆序关闭 | `internal/xclient`（多实例模块共用，不对外） |
| 单实例 / 多实例两种写法的解码 | `xconfig.UnmarshalClients` |
| 带总预算的重试 | `xutil.Retry` |
| 启动期建连探测：试几次、单次预算、认证被拒不再试（xgorm、xredis 共用） | `xclient.Probe`（基于 `xutil.Retry`） |
| 注册指标并断回具体类型 | `xmetric.RegisterAs` |

`C()` 取不到实例时直接 panic，而且说清是哪一种，因为四种要查的地方各不相同：

| 情况 | 文案里说的 |
|---|---|
| `xone.Run` 还没把它建起来（在 `main` 里、`init()` 里、更早的档位里取） | 调早了，该在 `Start`、请求处理或默认档（`StageBusiness`）及之后的钩子里用 |
| 退出流程已经把它关了 | 调晚了 |
| 配置里整块没写 | 没配，去看配置里的那一块 |
| 写了，但没有这个名字 | 名字写错，列出已配置的名字 |

`xhttp` 不连任何外部资源，任何时候 `C()` 都有一个可用的客户端（`Run` 之前是默认配置）。

## 模块之间怎么互相扩展

下层不认识上层。需要上层能力时，下层持有一个函数类型的扩展点，上层注入：

| 扩展点 | 注入方 | 作用 |
|---|---|---|
| `xlog.SetTraceExtractor` | xtrace | 日志自动带上 `trace_id`，而 xlog 不依赖 OpenTelemetry |
| `xlog.AddObserver` | xmetric | 统计错误日志条数，而 xlog 不依赖 Prometheus |

不用「在 `slog.Default()` 外面包一层」的办法：`slog.SetDefault` 会把标准库
`log` 包的输出也接到新 handler 上，链条一旦绕回 slog 自带的 handler 就成环，
卡死在 `log` 包那把不可重入的锁上。让下层自己持有扩展点就没有这个问题。

---

## 配置加载的几个决定

规则本身在 [`config.md`「通用规则」](config.md#通用规则)，这里只记为什么这样定：

- **默认值预填在结构体里，不用 `*bool` 指针。** 文件没写的字段保持不变，`Enable: false` 就是 false。
- **集合里的默认值。** 多实例的 `Clients` 由 `xconfig.UnmarshalClients` 逐个实例铺默认值。
  别的 map / 切片元素从零值开始解，要默认值就给元素类型写 `UnmarshalYAML`，里面用
  `xconfig.DecodeStrict`（**不能用 `node.Decode`，它会丢掉严格检查**）。
- **单实例与多实例不能混用。** `XGorm.DSN` 是单实例写法，`XGorm.Clients.<名字>` 是多实例写法。
- **拼错字段、没人认领的块都是启动失败。** 后者还提示可能是忘了 import 对应的包——
  两种情况都会让人配了半天才发现不生效，而配置文件是使用者唯一的操作界面。
- **占位符在解析后的节点上展开**，不是对原始字节做文本替换——否则环境变量的值里
  含冒号或换行就会改变 YAML 结构，那是一条注入路径。展开之后标量按新内容重新判定
  类型，所以 `Port: ${PORT:8080}` 填的是 int 字段；加了引号（`"${PW}"`）则固定
  按字符串处理，数字形态的密码、版本号靠这一条。
- **profile 与 import 的写法跟 Spring 一致**，从那边过来不用重新学；合并也一致：
  map 递归合并、列表整体替换、标量覆盖。列表这条最容易误解——逐元素合并的话
  `[A,B]` 叠上 `[C]` 会变成 `[C,B]`，你以为换掉了整张表，实际只换掉第一项。
- **一处故意和 Spring 不同**：点名的 profile 文件不存在时这里直接启动失败，
  Spring 是静默跳过。profile 名写错几乎总是笔误，静默跳过的结果是
  一份谁都没看过的配置悄悄以默认值起来。

---

## 我们故意跟底层库不一样的地方

框架的默认值是量过之后定的，有几处跟底层库自己的默认不同。
你熟悉这些库的话，这张表省得你被「怎么跟文档说的不一样」绊一下：

| 配置 | 库自己的默认 | 这里的默认 | 为什么 |
|---|---|---|---|
| `XGin.TrustedProxies` | 全都信（`0.0.0.0/0`） | 一个都不信 | 否则谁发 `X-Forwarded-For` 谁就是访问日志里的 `client_ip`，限流和审计跟着失效 |
| `XGin.MaxMultipartMemory` | 32MB | 8MB | 它是落盘阈值不是请求体上限，堆开销约为它的三倍：一次 60MB 的上传，32MB 要吃 96MB 堆 |
| `XGorm.Log: false` | 换成 GORM 自己的 stdout logger | 真的不打 | 那个默认实现带 ANSI 颜色直写 `os.Stdout`，绕开 slog 插进日志流 |
| `XRedis` 命令超时 | 只认 `ReadTimeout` | 听调用方 ctx 的截止时间 | 不然 200ms 预算的请求会在慢 Redis 上等满 `ReadTimeout`（实测 5s） 。ctx 的**取消**照样叫不醒阻塞的命令，见 docs/config.md 的 XRedis |
| `XRedis` 建连 | 每次建连内部重拨 5 次、间隔 100ms | 只拨一次，重试只有 `MaxRetries` 那一层 | 否则主机宕机时一条命令实测 11.7s，远超 `MaxRetries` 推出来的 7s |
| `XRedis` 的 go-redis 日志 | 标准库 log 写 `os.Stderr` | 接到 slog，记成 WARN | 绕开 slog 的日志进不了日志平台 |
| `XCache.MaxCost` | 每条另计 56 字节内部开销 | 只算你给的 cost | 否则 `MaxCost: 2000` 实际只存得下 35 条 |
| `XGorm.Log: true` | 参数值代进 SQL 再记 | 只记带占位符的 SQL | 否则 `WHERE password = ?` 记下来的是真实的密码 |
| `XRedis.Trace` | 整条命令连同参数写进 `db.statement` | 只有命令名 | 否则 `SET` 的值（会话、令牌）原样进链路后端 |
| `XCache` 停止 | `Close` | `Clear` | ristretto 的 `Close` 与并发的 `Set` / `Get` 一起跑会 panic，而原生 `*Cache` 谁还攥着框架不知道 |
| `XGorm` 建连 | `gorm.Open` 自己 ping 一次 | 关掉，走框架的 ctx-aware ping | 它用自己的 context，退出信号和重试都管不到 |
| `XGorm` MySQL 查版本 | `gorm.Open` 里用 `context.Background()` 查 `SELECT VERSION()` | 关掉，挪进建连探测 | 它是第一次建连：失败了不重试，启动期间的退出信号要等满 `ReadTimeout`（实测信号后 2.95s） |
| `XGorm` 的 go-sql-driver 日志 | 标准库 log 写 `os.Stderr` | 接到 slog，记成 WARN | 绕开 slog 的日志进不了日志平台 |
| `XGin.UseH2C` | x/net 的 `h2c.NewHandler` | 标准库的 `Protocols.SetUnencryptedHTTP2` | 前者劫持连接，`Shutdown` 约 60µs 就返回、在途请求照跑；只认先验知识的 h2c，不支持 `Upgrade: h2c` |
| `XGin.ReadHeaderTimeout: 0` | 退到 `ReadTimeout`（默认 0），即不限时 | 启动失败 | 发半个请求头就能一直占着连接 |
| `XHttp` 的 resty 日志 | 写 `os.Stderr`，重试失败时连查询串一起打 | 接到 slog，去掉查询串 | 绕开 slog 的日志进不了日志平台，令牌原样落盘 |
| `XHttp` 的 cookie | `resty.New()` 自带 cookie jar | 没有 | 不相干的调用会共享别人种下的会话 cookie |
| `XHttp` 出站 Span | otelhttp 的 `url.full` 带查询串 | 去掉查询串，Span 名只用方法 | 查询串里的令牌进链路后端；按路径起名的基数随 id 增长 |
| `XTrace` 透传 | 入站请求里的值照单全收 | 只收直连对端在 `XGin.TrustedProxies` 里的 | 否则公网客户端能伪造 `X-Tenant-Id` 这类头，被带进内网 |

每一项的具体数字和配置方法在 [`config.md`](config.md) 对应模块的一节里。

---

## 状态

| 波次 | 内容 | 状态 |
|---|---|---|
| 0 | 地基：xhook / config / 根包 / check.sh | 完成 |
| 1 | xapp / xlog / xtrace / xmetric | 完成 |
| 2 | xgorm / xredis / xcache / xhttp | 完成 |
| 3 | xgin 及中间件、xginswagger | 完成 |
| 4 | xflow、文档、CI、发布脚本 | 完成 |
| — | 打 v0.1.0 tag | 待人工确认 |

各模块都还没打 tag，子模块用 `replace` 指向仓库内的相对路径。
