package xkv

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/xiaoshicae/x-one/internal/config"
	"github.com/xiaoshicae/x-one/internal/hook"
)

// 这个文件也是样例的一部分：纯构造器 New 的回报就是测试里不需要任何 mock，
// 也不需要把框架拉起来——直接造一个干净实例，用完关掉。

func TestStore_写了能读回来(t *testing.T) {
	s, closer, err := New(Config{Path: filepath.Join(t.TempDir(), "kv.json"), FlushInterval: time.Hour})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { closer.Close() })

	s.Set("k", "v")
	if got, ok := s.Get("k"); !ok || got != "v" {
		t.Fatalf("写进去的该读得回来，got=%q ok=%v", got, ok)
	}
	if s.Len() != 1 {
		t.Errorf("该只有一个键，got=%d", s.Len())
	}
}

func TestStore_关闭时落盘下次能读回来(t *testing.T) {
	// 后台刷盘间隔调得很长，确保这次落盘只可能来自 Close
	path := filepath.Join(t.TempDir(), "kv.json")
	c := Config{Path: path, FlushInterval: time.Hour}

	s, closer, err := New(c)
	if err != nil {
		t.Fatal(err)
	}
	s.Set("k", "v")
	if err := closer.Close(); err != nil {
		t.Fatal(err)
	}

	again, closer2, err := New(c)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { closer2.Close() })
	if got, _ := again.Get("k"); got != "v" {
		t.Fatalf("关闭时该把改动刷下去，重开应当读得到，got=%q", got)
	}
}

func TestStore_数据文件坏掉时建不起来(t *testing.T) {
	// 建不起来就该报错，让启动当场失败——而不是静默地从一个空 store 开始，
	// 那会让线上看起来一切正常，只是数据没了
	path := filepath.Join(t.TempDir(), "kv.json")
	if err := os.WriteFile(path, []byte("not json"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, _, err := New(Config{Path: path, FlushInterval: time.Hour}); err == nil {
		t.Fatal("数据文件坏掉时应当报错")
	}
}

func TestRegister_登记了启停钩子(t *testing.T) {
	// 这是本包和框架之间唯一的一根线：钩子漏登记、档位挂错，
	// 表现是「配置不生效」或者「实例比用它的东西晚就绪」，别处都测不出来
	var got *hook.Entry
	for _, e := range hook.Start() {
		if e.Pkg == "github.com/xiaoshicae/x-one/example/component/xkv" {
			got = &e
		}
	}
	if got == nil {
		t.Fatal("没有登记启动钩子")
	}
	if got.Stage != hook.StageClient {
		t.Errorf("被业务依赖的东西该在 StageClient 就绪，got=%v", got.Stage)
	}

	var stopped bool
	for _, e := range hook.Stop() {
		if e.Pkg == "github.com/xiaoshicae/x-one/example/component/xkv" {
			stopped = true
			if e.Stage != hook.StageClient {
				// 启动和停止不在同一档的话，业务还在用的时候它就被关了
				t.Errorf("停止钩子也该在 StageClient，got=%v", e.Stage)
			}
		}
	}
	if !stopped {
		t.Error("有资源要关就必须登记停止钩子，否则退出时数据刷不下去")
	}
}

func TestInitXKV_走一遍框架真正会走的路(t *testing.T) {
	// 这是本例子存在的意义：读配置 → 建实例 → 存起来 → 退出时关掉。
	// 使用者照抄的就是这几行，它们必须真的串得起来
	dir := t.TempDir()
	path := filepath.Join(dir, "kv.json")
	useConf(t, "XKV:\n  Path: \""+path+"\"\n  FlushInterval: 50ms\n")

	if err := initXKV(context.Background()); err != nil {
		t.Fatalf("初始化失败：%v", err)
	}
	if C() == nil {
		t.Fatal("实例没存起来，C() 取不到")
	}
	if Conf().Path != path {
		t.Errorf("配置没读到，got=%+v", Conf())
	}

	C().Set("k", "v")
	if err := closeXKV(context.Background()); err != nil {
		t.Fatalf("关闭失败：%v", err)
	}

	// 关闭时把最后一次改动刷下去了，所以重新建一个能读回来
	s, closer, err := New(Conf())
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = closer.Close() })
	if got, _ := s.Get("k"); got != "v" {
		t.Errorf("退出前没落盘，got=%q", got)
	}
}

func TestInitXKV_配置写错时启动失败(t *testing.T) {
	useConf(t, "XKV:\n  Paht: /tmp/kv.json\n")
	if err := initXKV(context.Background()); err == nil {
		t.Fatal("字段拼错应当让启动失败")
	}
}

func TestNew_刷盘间隔不大于0时建不起来(t *testing.T) {
	// 放过去的话 time.NewTicker 在后台协程里 panic，进程直接死掉
	for _, every := range []time.Duration{0, -time.Second} {
		if _, _, err := New(Config{Path: filepath.Join(t.TempDir(), "kv.json"), FlushInterval: every}); err == nil {
			t.Errorf("FlushInterval=%v 应当建不起来", every)
		}
	}
}

func TestInitXKV_刷盘间隔配成0时启动失败(t *testing.T) {
	useConf(t, "XKV:\n  FlushInterval: 0s\n")
	if err := initXKV(context.Background()); err == nil {
		t.Fatal("FlushInterval 为 0 应当让启动失败")
	}
}

func TestStore_一次刷盘失败不丢数据(t *testing.T) {
	// 从前刷盘先清掉脏标记再写，写失败了也不还回去：这批改动再也没人刷，
	// 连 Close 时最后那一次也当成「没有改动」跳过
	dir := filepath.Join(t.TempDir(), "还不存在的目录")
	path := filepath.Join(dir, "kv.json")
	s, closer, err := New(Config{Path: path, FlushInterval: time.Hour})
	if err != nil {
		t.Fatal(err)
	}

	s.Set("k", "v")
	if err := s.flush(); err == nil {
		t.Fatal("目录不存在，这次刷盘应当失败")
	}
	if err := os.Mkdir(dir, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := closer.Close(); err != nil {
		t.Fatalf("目录建好之后 Close 应当刷得下去：%v", err)
	}

	again, closer2, err := New(Config{Path: path, FlushInterval: time.Hour})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = closer2.Close() })
	if got, _ := again.Get("k"); got != "v" {
		t.Errorf("刷盘失败过一次的改动丢了，got=%q", got)
	}
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
