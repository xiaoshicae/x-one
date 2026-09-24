package xapp

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/xiaoshicae/x-one/internal/config"
	"github.com/xiaoshicae/x-one/internal/hook"
)

func TestRegister_只读配置不建任何东西(t *testing.T) {
	var got *hook.Entry
	for _, e := range hook.Start() {
		if e.Pkg == "github.com/xiaoshicae/x-one/xapp" {
			got = &e
		}
	}
	if got == nil {
		t.Fatal("没有登记启动钩子，App 那一块就没人读")
	}
	if got.Stage != hook.StageLog {
		t.Errorf("链路要拿 App.Name 当服务名，所以这一块必须更早读好，got=%v", got.Stage)
	}
	for _, e := range hook.Stop() {
		if e.Pkg == "github.com/xiaoshicae/x-one/xapp" {
			t.Error("本包没有要关的资源，登记停止钩子会让它出现在退出序列里")
		}
	}
}

func TestNameVersion(t *testing.T) {
	old := cfg
	t.Cleanup(func() { cfg = old })

	cfg = DefaultConfig()
	if Name() != "" || Version() != "" {
		t.Errorf("默认应为空，got=%q %q", Name(), Version())
	}
	cfg = Config{Name: "xone.demo.app", Version: "v1.2.0"}
	if Name() != "xone.demo.app" || Version() != "v1.2.0" {
		t.Errorf("读到的应是配置里的值，got=%q %q", Name(), Version())
	}
}

// useConf 把一份配置装进全局配置，走的是框架真正会走的那条路
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

func keepCfg(t *testing.T) {
	t.Helper()
	old := cfg
	t.Cleanup(func() { cfg = old })
}

func TestLoadConfig_读的是_App_这一块(t *testing.T) {
	// 服务名和版本号会被链路和指标当成 service.name / service.version，
	// 这一块没读到的话，面板上整个服务就是匿名的
	keepCfg(t)
	cfg = DefaultConfig()
	useConf(t, "App:\n  Name: xone.demo.app\n  Version: v1.2.0\n")

	if err := loadConfig(context.Background()); err != nil {
		t.Fatal(err)
	}
	if Name() != "xone.demo.app" || Version() != "v1.2.0" {
		t.Errorf("配置没读进来，got=%q %q", Name(), Version())
	}
}

func TestLoadConfig_没配就保持空值(t *testing.T) {
	keepCfg(t)
	cfg = DefaultConfig()
	useConf(t, "XLog:\n  Level: info\n")

	if err := loadConfig(context.Background()); err != nil {
		t.Fatalf("没配不该报错：%v", err)
	}
	if Name() != "" || Version() != "" {
		t.Errorf("没配时应为空，got=%q %q", Name(), Version())
	}
}

func TestLoadConfig_配置写错时启动失败(t *testing.T) {
	keepCfg(t)
	useConf(t, "App:\n  Nmae: demo\n")

	if err := loadConfig(context.Background()); err == nil {
		t.Fatal("字段拼错应当让启动失败，否则服务名会一直是空的而没人知道")
	}
}

func TestLoadConfig_还没加载时先加载再读(t *testing.T) {
	// 读得早拿到的也是文件里的值，不是一份静默的默认值
	keepCfg(t)
	p := filepath.Join(t.TempDir(), "application.yml")
	if err := os.WriteFile(p, []byte("App:\n  Name: early\n"), 0o600); err != nil {
		t.Fatal(err)
	}
	config.Reset()
	t.Cleanup(config.Reset)
	t.Setenv(config.EnvKey, p)

	if err := loadConfig(context.Background()); err != nil {
		t.Fatal(err)
	}
	if Name() != "early" {
		t.Errorf("读到的应是文件里的值，got=%q", Name())
	}
}
