// Package xconfig 提供集成包写配置解码时需要的那一点东西。
//
// 框架读配置的规矩是「默认值预填在结构体里、未知字段是错误」。
// 结构体字段上这两条都自动成立，但**集合装不下默认值**：
// map 的 value 和切片元素都是从零值开始解的，框架不知道该拿什么去填。
//
// 多实例的 Clients 块不用操这个心：DecodeClients 拿你给的默认值逐个实例铺好再解。
// 别的集合要默认值，就给元素类型写一个 UnmarshalYAML，先铺默认值再解：
//
//	func (c *ClientConfig) UnmarshalYAML(n *yaml.Node) error {
//		*c = DefaultClientConfig()
//		type raw ClientConfig // 换个类型，否则这里会递归调用自己
//		return xconfig.DecodeStrict(n, (*raw)(c))
//	}
//
// 这里必须用 DecodeStrict 而不是 n.Decode：后者不带严格检查，
// 于是「字段拼错就启动失败」这条保证会在集合里悄悄失效。
package xconfig

import (
	"fmt"
	"strings"

	"go.yaml.in/yaml/v3"

	"github.com/xiaoshicae/x-one/internal/config"
	"github.com/xiaoshicae/x-one/xerror"
)

// Unmarshal 把配置文件里 key 那一块解进 into。
//
//	func initXRedis(ctx context.Context) error {
//		c := DefaultConfig()
//		if err := xconfig.Unmarshal(ConfigKey, &c); err != nil {
//			return err
//		}
//		...
//	}
//
// into 里已经是默认值，文件里没写的字段保持不变——所以不需要指针字段来区分
// 「没配」和「配成零值」。整块没配时 into 原样不动，返回 nil；要区分
// 「没配」和「配了」用 Has。
//
// 认不出的字段是错误。into 若实现了 Validate() error，解完会调一次。
//
// **什么时候调都行**：在 main 里、在 xone.Run 之前、在钩子里都一样。
// 第一次调用时框架才去找配置文件、加载它，读到的永远是最终值——不会因为
// 读得早就静默拿到一份默认值。
func Unmarshal(key string, into any) error { return config.Unmarshal(key, into) }

// Has 报告配置文件里有没有写这一块，用于「配了才初始化」。
//
//	func initXRedis(ctx context.Context) error {
//		if !xconfig.Has(ConfigKey) {
//			return nil // 没配就不建，本模块是可选依赖
//		}
//		...
//	}
//
// 和 Unmarshal 一样，问过这一块就算你认领了它：框架在全部启动钩子跑完之后
// 会把没人认领的顶层 key 报出来，跳过的那些不该落在那张名单里。
func Has(key string) bool { return config.Has(key) }

// DecodeStrict 把一个 YAML 节点解进 v，认不出的字段是错误。
func DecodeStrict(node *yaml.Node, v any) error { return config.DecodeStrict(node, v) }

// HasKey 报告一个 mapping 节点里有没有这个 key。
//
// 用于按配置的形状分派——比如同一个块既支持单实例也支持多实例时，
// 看有没有 Clients 决定按哪种解。
func HasKey(node *yaml.Node, key string) bool {
	if node == nil || node.Kind != yaml.MappingNode {
		return false
	}
	for i := 0; i+1 < len(node.Content); i += 2 {
		if node.Content[i].Value == key {
			return true
		}
	}
	return false
}

// DecodeClients 解一个「既支持单实例也支持多实例」的配置块。
//
//	XGorm:                  # 单实例，直接写字段，名字就是 default
//	  DSN: "${DB_DSN}"
//
//	XGorm:                  # 多实例，按名字写
//	  Clients:
//	    default: {DSN: "${DB_DSN}"}
//	    report:  {DSN: "${REPORT_DSN}"}
//
// 看有没有 Clients 决定按哪种解。两种混着写直接报错：那时候
// 「default 到底是哪个」没有一个不让人意外的答案。
//
// defaults 提供单个实例的默认值，两种写法里的每个实例都先铺上它再解，
// 所以文件里没写的字段保持默认——元素类型不必自己写 UnmarshalYAML。
func DecodeClients[C any](n *yaml.Node, defaults func() C) (map[string]C, error) {
	if !HasKey(n, clientsKey) {
		single := defaults()
		if err := DecodeStrict(n, &single); err != nil {
			return nil, err
		}
		return map[string]C{DefaultClientName: single}, nil
	}

	// 混用先自己认出来，别指望从解码错误里读。
	// 交给解码器的话，实例里一个字段拼错（Clients.default.DSNN）报的也是
	// 「不能混用」——而那份配置根本没混用，使用者会照着这句话去改一个没问题的地方。
	if stray := keysExcept(n, clientsKey); len(stray) > 0 {
		return nil, xerror.Newf("xconfig", "config", "cannot mix the single- and multi-instance forms: with %s present, "+
			"%s belong to no instance — move them into one, or drop %s and use the single-instance form",
			clientsKey, strings.Join(stray, ", "), clientsKey)
	}

	// 逐个实例铺默认值再解：直接解进 map[string]C 的话，map 的 value 是从零值
	// 开始的，没写的字段全成了零值。
	//
	// 实例的节点直接从 n 里取，不先解进 map[string]yaml.Node：那样拿到的是
	// 重新序列化出来的节点，报错的行号就和 n 对不上，也换不回配置文件里的那一行
	list := valueOf(n, clientsKey)
	if list == nil || list.ShortTag() == "!!null" || (list.Kind == yaml.MappingNode && len(list.Content) == 0) {
		return nil, xerror.Newf("xconfig", "config", "%s is empty: either list instances under it, or remove the whole block", clientsKey)
	}
	if list.Kind != yaml.MappingNode {
		return nil, &yaml.TypeError{Errors: []string{
			fmt.Sprintf("line %d: %s must map instance names to their config", list.Line, clientsKey)}}
	}

	// 字段和类型层面的问题收齐了一起报，并且仍是 *yaml.TypeError：
	// 这里多半跑在某个 UnmarshalYAML 里，yaml 只把这种错误并进外层的报错列表，
	// 外层的 DecodeStrict 才能接着把行号换回配置文件里的那一行
	var bad []string
	out := make(map[string]C, len(list.Content)/2)
	for i := 0; i+1 < len(list.Content); i += 2 {
		name, node := list.Content[i].Value, list.Content[i+1]
		if _, dup := out[name]; dup {
			return nil, xerror.Newf("xconfig", "config", "%s.%s is defined twice", clientsKey, name)
		}
		c := defaults()
		// 只写了名字（report: 后面是空的）就是全用默认值
		if node.ShortTag() != "!!null" {
			if err := DecodeStrict(node, &c); err != nil {
				te, ok := err.(*yaml.TypeError)
				if !ok {
					return nil, xerror.Newf("xconfig", "config", "%s.%s: %w", clientsKey, name, err)
				}
				bad = append(bad, within(te, clientsKey+"."+name)...)
			}
		}
		out[name] = c
	}
	if len(bad) > 0 {
		return nil, &yaml.TypeError{Errors: bad}
	}
	return out, nil
}

// valueOf mapping 里 key 对应的值，没有则返回 nil
func valueOf(node *yaml.Node, key string) *yaml.Node {
	for i := 0; i+1 < len(node.Content); i += 2 {
		if node.Content[i].Value == key {
			return node.Content[i+1]
		}
	}
	return nil
}

// within 给每一条类型错误点名是哪个实例，行号留在开头，外层还要认它
func within(te *yaml.TypeError, where string) []string {
	out := make([]string, len(te.Errors))
	for i, e := range te.Errors {
		if head, rest, ok := strings.Cut(e, ": "); ok && strings.HasPrefix(head, "line ") {
			out[i] = head + ": " + where + ": " + rest
		} else {
			out[i] = where + ": " + e
		}
	}
	return out
}

// keysExcept 列出 mapping 里除 except 之外的 key，保持书写顺序
func keysExcept(node *yaml.Node, except string) []string {
	if node == nil || node.Kind != yaml.MappingNode {
		return nil
	}
	var out []string
	for i := 0; i+1 < len(node.Content); i += 2 {
		if k := node.Content[i].Value; k != except {
			out = append(out, k)
		}
	}
	return out
}

const (
	clientsKey = "Clients"

	// DefaultClientName 单实例写法被规整成的名字。
	// 与 xclient.DefaultName 一致；这里再写一遍是为了不让核心的配置包
	// 反过来依赖 xclient。
	DefaultClientName = "default"
)
