package xconfig

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.yaml.in/yaml/v3"

	"github.com/xiaoshicae/x-one/internal/config"
)

type item struct {
	A string `yaml:"A"`
	B int    `yaml:"B"`
}

func defItem() item { return item{B: 42} }

// UnmarshalYAML 就是本包文档里推荐的写法，顺带当例子测一遍
func (i *item) UnmarshalYAML(n *yaml.Node) error {
	*i = defItem()
	type raw item
	return DecodeStrict(n, (*raw)(i))
}

func decode(t *testing.T, src string, v any) error {
	t.Helper()
	var n yaml.Node
	if err := yaml.Unmarshal([]byte(src), &n); err != nil {
		t.Fatal(err)
	}
	return DecodeStrict(n.Content[0], v)
}

func TestDecodeStrict_未知字段是错误(t *testing.T) {
	var got item
	err := decode(t, "A: x\nZZZ: 1\n", &got)
	if err == nil {
		t.Fatal("拼错的字段应当报错")
	}
	if !strings.Contains(err.Error(), "ZZZ") {
		t.Errorf("错误里要点名，got=%v", err)
	}
}

func TestDecodeStrict_集合元素也保留严格检查(t *testing.T) {
	// 这是本包存在的理由：元素类型自己写 UnmarshalYAML 铺默认值时，
	// 如果图省事用 node.Decode，严格检查就在集合里悄悄失效了
	var got map[string]item
	if err := decode(t, "a:\n  A: x\n  ZZZ: 1\n", &got); err == nil {
		t.Fatal("集合元素里的字段拼错也应当报错")
	}
}

func TestDecodeStrict_默认值铺得上(t *testing.T) {
	var got map[string]item
	if err := decode(t, "a:\n  A: x\n", &got); err != nil {
		t.Fatal(err)
	}
	if got["a"].A != "x" || got["a"].B != 42 {
		t.Errorf("没写的字段应保持默认，got=%+v", got["a"])
	}
}

func TestHasKey(t *testing.T) {
	var n yaml.Node
	if err := yaml.Unmarshal([]byte("A: 1\nClients:\n  x: 1\n"), &n); err != nil {
		t.Fatal(err)
	}
	doc := n.Content[0]

	if !HasKey(doc, "Clients") || !HasKey(doc, "A") {
		t.Error("有的 key 应当认得出来")
	}
	if HasKey(doc, "Nope") {
		t.Error("没有的 key 不该认成有")
	}
	if HasKey(nil, "A") {
		t.Error("nil 节点不该 panic，也不该说有")
	}

	var scalar yaml.Node
	yaml.Unmarshal([]byte("just-a-string\n"), &scalar)
	if HasKey(scalar.Content[0], "A") {
		t.Error("不是 mapping 的节点不该说有 key")
	}
}

type client struct {
	Addr string `yaml:"Addr"`
	Max  int    `yaml:"Max"`
}

// client 故意不写 UnmarshalYAML：两种写法的默认值都该由 DecodeClients 铺
func defClient() client { return client{Addr: "127.0.0.1", Max: 50} }

func clients(t *testing.T, src string) (map[string]client, error) {
	t.Helper()
	var n yaml.Node
	if err := yaml.Unmarshal([]byte(src), &n); err != nil {
		t.Fatal(err)
	}
	return DecodeClients(n.Content[0], defClient)
}

func TestDecodeClients_单实例写法(t *testing.T) {
	got, err := clients(t, "Addr: 10.0.0.1\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 {
		t.Fatalf("应解出一个实例，got=%v", got)
	}
	c, ok := got[DefaultClientName]
	if !ok {
		t.Fatalf("单实例写法应规整成名为 %s 的实例，got=%v", DefaultClientName, got)
	}
	if c.Addr != "10.0.0.1" || c.Max != 50 {
		t.Errorf("写了的生效、没写的保持默认，got=%+v", c)
	}
}

func TestDecodeClients_多实例写法(t *testing.T) {
	got, err := clients(t, "Clients:\n  default: {Addr: a}\n  report: {Addr: b, Max: 5}\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 {
		t.Fatalf("应解出两个实例，got=%v", got)
	}
	// 一个实例覆盖了的字段不该影响另一个
	if got["default"].Max != 50 || got["report"].Max != 5 {
		t.Errorf("默认值应逐个实例生效，got=%v", got)
	}
}

func TestDecodeClients_多实例只写了名字也拿到默认值(t *testing.T) {
	// report: 后面什么都没写，是「这个实例全用默认值」，不是「这个实例全是零值」
	got, err := clients(t, "Clients:\n  default: {Addr: a}\n  report:\n")
	if err != nil {
		t.Fatal(err)
	}
	if got["report"] != defClient() {
		t.Errorf("没写的实例该是默认值，got=%+v", got["report"])
	}
}

func TestDecodeClients_实例里拼错时点名是哪个实例(t *testing.T) {
	_, err := clients(t, "Clients:\n  a: {Addr: x}\n  b: {Adrr: y}\n")
	if err == nil {
		t.Fatal("拼错应当报错")
	}
	if !strings.Contains(err.Error(), "Clients.b") || !strings.Contains(err.Error(), "Adrr") {
		t.Errorf("错误该点名实例和字段，got=%v", err)
	}
}

func TestDecodeClients_两种写法不能混用(t *testing.T) {
	// 混着写时「default 到底是哪个」没有不让人意外的答案
	_, err := clients(t, "Addr: a\nClients:\n  x: {Addr: b}\n")
	if err == nil {
		t.Fatal("混用两种写法应当报错")
	}
	if !strings.Contains(err.Error(), "cannot mix") {
		t.Errorf("错误该说清楚为什么，got=%v", err)
	}
	// 还要点名是哪个字段没了归属，否则一个几十行的配置块看不出改哪里
	if !strings.Contains(err.Error(), "Addr") {
		t.Errorf("错误该指出是哪个字段，got=%v", err)
	}
}

func TestDecodeClients_实例里拼错不该被报成混用(t *testing.T) {
	// 「不能混用」这句话曾经是套在任何一个解码错误上的：实例里一个字段拼错
	// （Clients.x.Adrr）报的也是它。那份配置根本没混用，使用者会照着这句话
	// 去改一个没问题的地方，真正的拼写错误反而被这句提示盖住了。
	cases := map[string]string{
		"实例里字段拼错": "Clients:\n  x: {Adrr: a}\n",
		"实例里类型不对": "Clients:\n  x: {Max: abc}\n",
	}
	for name, src := range cases {
		_, err := clients(t, src)
		if err == nil {
			t.Fatalf("%s：应当报错", name)
		}
		if strings.Contains(err.Error(), "cannot mix") {
			t.Errorf("%s：这不是混用，不该这么报，got=%v", name, err)
		}
	}
}

func TestDecodeClients_空的Clients要报错(t *testing.T) {
	if _, err := clients(t, "Clients: {}\n"); err == nil {
		t.Fatal("写了 Clients 却是空的，应当报错")
	}
}

func TestDecodeClients_拼写错误两种写法都要拦住(t *testing.T) {
	// 集合元素走的是自定义解码器，严格检查很容易在那里悄悄失效
	if _, err := clients(t, "Adrr: a\n"); err == nil {
		t.Error("单实例写法里拼错应当报错")
	}
	if _, err := clients(t, "Clients:\n  x: {Adrr: a}\n"); err == nil {
		t.Error("多实例写法里拼错应当报错")
	}
}

// ---- Unmarshal / Has：集成包唯一会用到的两个入口 ----

type modCfg struct {
	Addr    string `yaml:"Addr"`
	Retry   int    `yaml:"Retry"`
	badFlag bool
}

func (c *modCfg) Validate() error {
	if c.badFlag {
		return errors.New("validate said no")
	}
	return nil
}

// useConf 把一份配置装进全局配置
func useConf(t *testing.T, yml string) {
	t.Helper()
	p := filepath.Join(t.TempDir(), "application.yml")
	if err := os.WriteFile(p, []byte(yml), 0o600); err != nil {
		t.Fatal(err)
	}
	config.Reset()
	if err := config.Load(p); err != nil {
		t.Fatalf("加载配置失败：%v", err)
	}
	t.Cleanup(config.Reset)
}

func TestUnmarshal_文件里没写的字段保持默认值(t *testing.T) {
	// 「默认值预填在结构体里」是整个配置模型的地基：少了它，
	// 每个字段都得用指针类型来区分「没配」和「配成零值」
	useConf(t, "XMod:\n  Retry: 3\n")

	c := modCfg{Addr: "127.0.0.1", Retry: 1}
	if err := Unmarshal("XMod", &c); err != nil {
		t.Fatal(err)
	}
	if c.Addr != "127.0.0.1" {
		t.Errorf("没写的字段被抹成零值了，got=%q", c.Addr)
	}
	if c.Retry != 3 {
		t.Errorf("写了的字段没生效，got=%d", c.Retry)
	}
}

func TestUnmarshal_整块没配时原样不动(t *testing.T) {
	useConf(t, "App:\n  Name: demo\n")

	c := modCfg{Addr: "127.0.0.1"}
	if err := Unmarshal("XMod", &c); err != nil {
		t.Fatalf("没配这一块不该报错：%v", err)
	}
	if c.Addr != "127.0.0.1" {
		t.Errorf("默认值被动了，got=%q", c.Addr)
	}
}

func TestUnmarshal_未知字段是错误(t *testing.T) {
	useConf(t, "XMod:\n  Adr: 127.0.0.1\n")

	c := modCfg{}
	if err := Unmarshal("XMod", &c); err == nil {
		t.Fatal("字段拼错应当报错，否则使用者会一直以为自己配上了")
	}
}

func TestUnmarshal_会调一次_Validate(t *testing.T) {
	// 有些配错不会让初始化失败，只是让某个行为永远走不到，
	// 那种只能靠 Validate 拦
	useConf(t, "XMod:\n  Retry: 1\n")

	c := modCfg{badFlag: true}
	err := Unmarshal("XMod", &c)
	if err == nil {
		t.Fatal("Validate 报错应当让 Unmarshal 失败")
	}
	if !strings.Contains(err.Error(), "validate said no") {
		t.Errorf("要把 Validate 的话带上来，got=%v", err)
	}
}

// useEnvConf 把一份配置写进文件，经 XONE_CONFIG 交给「第一次读时自动加载」
func useEnvConf(t *testing.T, yml string) {
	t.Helper()
	p := filepath.Join(t.TempDir(), "application.yml")
	if err := os.WriteFile(p, []byte(yml), 0o600); err != nil {
		t.Fatal(err)
	}
	config.Reset()
	t.Cleanup(config.Reset)
	t.Setenv(config.EnvKey, p)
}

func TestUnmarshal_还没加载时先加载(t *testing.T) {
	// 读得早不是错误，也不会静默拿到默认值：第一次读就先加载，
	// 在 main 里、在 Run 之前读到的都是文件里的最终值
	useEnvConf(t, "XMod:\n  Retry: 7\n")

	c := modCfg{}
	if err := Unmarshal("XMod", &c); err != nil {
		t.Fatal(err)
	}
	if c.Retry != 7 {
		t.Errorf("读到的应是文件里的值，got=%d", c.Retry)
	}
}

func TestHas_区分没配和配了(t *testing.T) {
	useConf(t, "XMod:\n  Retry: 1\nXOther:\n")

	if !Has("XMod") {
		t.Error("写了的块应当报告有")
	}
	if Has("XNope") {
		t.Error("没写的块不该报告有")
	}
	if Has("XOther") {
		t.Error("写了个空块等于没配，不该让可选组件白建一套")
	}
}

func TestHas_还没加载时先加载(t *testing.T) {
	useEnvConf(t, "XMod:\n  Retry: 1\n")
	if !Has("XMod") {
		t.Error("第一次问的时候该先加载，而不是报告没有")
	}
}

func TestHas_问过就算认领(t *testing.T) {
	// 跳过的那一块不该落在「没人认领」的名单里：可选组件正是靠 Has
	// 决定跳过的，把它算成没人要会让启动直接失败
	useConf(t, "XMod:\n")

	if Has("XMod") {
		t.Fatal("空块应当报告没配")
	}
	if got := config.Unclaimed(); len(got) != 0 {
		t.Errorf("want 空，got=%v", got)
	}
}

// multi 走 DecodeClients 的配置块，和 xredis 这些集成的写法一样
type multi struct{ m map[string]client }

func (c *multi) UnmarshalYAML(n *yaml.Node) (err error) {
	c.m, err = DecodeClients(n, defClient)
	return err
}

func TestDecodeClients_实例里拼错时报出配置文件和那一行(t *testing.T) {
	// 实例的节点从前先解进 map[string]yaml.Node，那是又一次重新序列化出来的节点，
	// 行号和配置文件对不上
	p := filepath.Join(t.TempDir(), "application.yml")
	body := "XMod:\n  Clients:\n    a: {Addr: x}\n    b:\n      Max: 1\n      Adrr: y\n"
	if err := os.WriteFile(p, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	config.Reset()
	t.Cleanup(config.Reset)
	if err := config.Load(p); err != nil {
		t.Fatal(err)
	}
	var c multi
	err := Unmarshal("XMod", &c)
	want := p + ":6: Clients.b: field Adrr not found"
	if err == nil || !strings.Contains(err.Error(), want) {
		t.Errorf("报错该点名文件、行号和实例 %q，got=%v", want, err)
	}
}
