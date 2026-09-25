# xconfig —— 读自己的配置

把配置文件里你自己的那一块解进一个普通的 Go 结构体（核心模块）。规则和框架自己的配置块一样：默认值预填、
字段拼错启动失败、`${VAR}` 照样展开、`Validate()` 在启动时就跑。

## 快速上手

```go
// main.go
package main

import (
	"errors"
	"log"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xconfig"
	"github.com/xiaoshicae/x-one/xgin"
)

// OrderConfig 对应配置文件里的 Order 块
type OrderConfig struct {
	PayTimeout  time.Duration `yaml:"PayTimeout"` // 每个字段都写 yaml tag
	MaxItems    int           `yaml:"MaxItems"`
	CallbackURL string        `yaml:"CallbackURL"`
}

// Validate 解完自动调一次：配错的值在启动时就失败，一个请求都还没接
func (c OrderConfig) Validate() error {
	if c.MaxItems <= 0 {
		return errors.New("MaxItems must be > 0")
	}
	return nil
}

func main() {
	cfg := OrderConfig{PayTimeout: 15 * time.Minute, MaxItems: 50} // 默认值预填：文件里没写的字段保持不变
	if err := xconfig.Unmarshal("Order", &cfg); err != nil {
		log.Fatal(err)
	}

	xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
		e.GET("/api/v1/order-limits", func(c *gin.Context) {
			c.JSON(http.StatusOK, gin.H{"max_items": cfg.MaxItems, "pay_timeout": cfg.PayTimeout.String()})
		})
	}))
}
```

```yaml
# conf/application.yml
Order:
  PayTimeout: 30m                        # 时长写单位：30s / 1500ms / 1h30m
  CallbackURL: "${ORDER_CALLBACK_URL}"   # 必填的环境变量，没设就启动失败
XGin:
  Port: 8080
```

`MaxItems` 没写，保持默认的 50；`PayTimeout` 被文件改成 30 分钟。

## 重点

- **每个字段都写 `yaml` tag。** 没写 tag 的字段 yaml.v3 只认全小写的 key，`PayTimeout:` 会被当成不认识的字段、启动失败——
  报错里会提示你加 tag，见 [troubleshooting.md](../docs/troubleshooting.md#field-bogus-not-found-in-type-xginconfig)。
- **默认值写在结构体里**，文件里没写的字段保持不变；整块没写时结构体原样不动、返回 nil。要区分「没配」和「配了」用
  `xconfig.Has("Order")`。
- **在 `Start` 之前什么时候读都行**：`main` 里、`xone.Run` 之前，或者一个 `BeforeStart` 钩子里（见[下文](#在钩子里读)）。
  第一次读的时候框架才去找、去加载配置文件，读到的永远是最终值。**别只在 `Start` 里读**：全部启动钩子跑完时还没人读过的
  顶层 key 会让启动失败（`config keys [Order] are not read by anyone`）。
- **写错就失败**：不认识的字段、`${VAR}` 没设、时长写成裸数字（`30` 会被当成 30 纳秒，所以直接报错）、`Validate()` 不通过，
  都在启动时报出来。
- 在 `main` 里读还有一个好处：「哪来的配置」由 `main` 决定，你的类型不必认识 `xconfig`，测试里直接传值。

## 在钩子里读

配置要在好几个包里用、或者读完还要做点初始化的，放进一个启动钩子。不写档位就是 `StageBusiness`：
所有客户端都已就绪、服务还没接流量。

```go
package order

import (
	"context"
	"time"

	"github.com/xiaoshicae/x-one/xconfig"
	"github.com/xiaoshicae/x-one/xhook"
)

type Config struct {
	PayTimeout time.Duration `yaml:"PayTimeout"`
}

var conf = Config{PayTimeout: 15 * time.Minute}

func init() {
	xhook.BeforeStart(func(ctx context.Context) error { // init 里只登记，由 xone.Run 按档位执行
		return xconfig.Unmarshal("Order", &conf)
	})
}

// C 在 Start、请求处理里用：那时启动钩子已经跑完
func C() Config { return conf }
```

可运行的样例：[`example/component/conf/`](../example/component/conf/)（在 `main` 里读）、[`example/consumer/`](../example/consumer/)。

## 配置文件的几条规则

一行一条，细节都在 [`docs/config.md`](../docs/config.md)：

| 想要 | 写法 | 详见 |
|---|---|---|
| 从环境变量取值 | `${VAR}` 必填，未设置启动失败；`${VAR:default}` 可选 | [占位符](../docs/config.md#占位符) |
| 按环境分文件 | `application-prod.yml` 放差异，`--profile=prod` 或 `XONE_PROFILE=prod` 选 | [Profiles](../docs/config.md#profiles--按环境分文件) |
| 把配置拆成几个文件 | `Import: [shared.yml, optional:local.yml]` | [Import](../docs/config.md#import--引入别的配置文件) |
| 指定配置文件 | `--config=<path>` 或 `XONE_CONFIG`；不指定就找 `conf/application.yml` 等约定路径 | [文件位置](../docs/config.md#文件位置与优先级) |
| 知道几个文件怎么叠 | map 递归合并，列表整体替换，标量覆盖 | [合并规则](../docs/config.md#合并规则) |

## 进阶

- **只在配了时才做**：`if xconfig.Has("Coupon") { … }`。问过就算认领，不会被当成没人读的 key。
- **map / 切片的元素也要默认值**：给元素类型写 `UnmarshalYAML`，先铺默认值、再用 `xconfig.DecodeStrict` 解
  （不能用 `node.Decode`，它会丢掉「字段拼错就失败」的严格检查）：

  ```go
  func (r *Rule) UnmarshalYAML(n *yaml.Node) error {
  	*r = DefaultRule()
  	type raw Rule // 换个类型，否则这里会递归调用自己
  	return xconfig.DecodeStrict(n, (*raw)(r))
  }
  ```

- **像 XGorm 那样的单实例 / 多实例两种写法**：`xconfig.UnmarshalClients(key, defaults)` 返回 `map[名字]配置`，看有没有
  `Clients` 决定按哪种解，两种混着写是错误，每个实例都先铺上默认值。主要给写集成的人用，见
  [guide.md「写一个自己的集成」](../docs/guide.md#写一个自己的集成)。
- **测试里换一份配置**：`xonetest.UseConfigYAML(t, yml)`，见 [guide.md「测试」](../docs/guide.md#测试)。
