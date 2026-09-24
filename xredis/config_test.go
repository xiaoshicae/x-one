package xredis

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"

	"github.com/xiaoshicae/x-one/internal/config"
)

func load(t *testing.T, yml string) Config {
	t.Helper()
	c := DefaultConfig()
	path := filepath.Join(t.TempDir(), "application.yml")
	if err := os.WriteFile(path, []byte(yml), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := config.LoadInto(path, ConfigKey, &c); err != nil {
		t.Fatalf("加载失败：%v", err)
	}
	return c
}

func loadErr(t *testing.T, yml string) error {
	t.Helper()
	c := DefaultConfig()
	path := filepath.Join(t.TempDir(), "application.yml")
	os.WriteFile(path, []byte(yml), 0o644)
	return config.LoadInto(path, ConfigKey, &c)
}

func TestConfig_单实例写法(t *testing.T) {
	c := load(t, "XRedis:\n  Addr: 10.0.0.1:6379\n  DB: 3\n")
	got, ok := c.Clients[DefaultName]
	if !ok {
		t.Fatalf("单实例写法应规整成名为 %s 的实例，got=%v", DefaultName, c.Clients)
	}
	if got.Addr != "10.0.0.1:6379" || got.DB != 3 {
		t.Errorf("配置不对，got=%+v", got)
	}
	if got.MinIdleConns != 5 || got.ConnMaxLifetime != 5*time.Minute || !got.Trace {
		t.Errorf("没写的字段应保持默认，got=%+v", got)
	}
}

func TestConfig_多实例写法(t *testing.T) {
	c := load(t, `
XRedis:
  Clients:
    default:
      Addr: 10.0.0.1:6379
    cache:
      Addr: 10.0.0.2:6379
      DB: 1
      Trace: false
`)
	if len(c.Clients) != 2 {
		t.Fatalf("应解出两个实例，got=%v", c.Clients)
	}
	if !c.Clients["default"].Trace {
		t.Error("一个实例关掉 Trace 不该影响另一个")
	}
	if cache := c.Clients["cache"]; cache.DB != 1 || cache.Trace {
		t.Errorf("写了的字段应生效，got=%+v", cache)
	}
}

func TestConfig_集合元素里的拼写错误也要失败(t *testing.T) {
	err := loadErr(t, "XRedis:\n  Clients:\n    default:\n      Adrr: x\n")
	if err == nil {
		t.Fatal("实例里的字段拼错应当启动失败")
	}
	if !strings.Contains(err.Error(), "Adrr") {
		t.Errorf("错误里要点名拼错的字段，got=%v", err)
	}
}

func TestConfig_两种写法不能混用(t *testing.T) {
	if err := loadErr(t, "XRedis:\n  Addr: x\n  Clients:\n    a:\n      Addr: y\n"); err == nil {
		t.Fatal("混用两种写法应当失败")
	}
}

func TestConfig_空的Clients要失败(t *testing.T) {
	if err := loadErr(t, "XRedis:\n  Clients: {}\n"); err == nil {
		t.Fatal("写了 Clients 却是空的，应当失败")
	}
}

func TestConfig_没配就没有实例(t *testing.T) {
	if c := load(t, "# 没有 XRedis 这一块\n"); len(c.Clients) != 0 {
		t.Errorf("没配就不该连任何 Redis，got=%v", c.Clients)
	}
}

func TestConfig_密码走环境变量(t *testing.T) {
	// 凭证不该进版本库
	t.Setenv("TEST_REDIS_PASSWORD", "hunter2")
	c := load(t, "XRedis:\n  Addr: h:6379\n  Password: \"${TEST_REDIS_PASSWORD}\"\n")
	if c.Clients[DefaultName].Password != "hunter2" {
		t.Errorf("环境变量应展开，got=%+v", c.Clients[DefaultName])
	}
}

func TestConfig_漏配必填的环境变量就启动失败(t *testing.T) {
	os.Unsetenv("TEST_REDIS_PASSWORD_MISSING")
	err := loadErr(t, "XRedis:\n  Password: \"${TEST_REDIS_PASSWORD_MISSING}\"\n")
	if err == nil {
		t.Fatal("凭证漏配应当启动失败，而不是静默变成空串")
	}
}

func TestConfig_重试配负数是关闭而不是没配(t *testing.T) {
	// go-redis 用 -1 表示「关掉」，0 表示「用默认」，两者都得传得下去
	c := load(t, "XRedis:\n  Addr: h:6379\n  MaxRetries: -1\n  MinRetryBackoff: -1ns\n")
	got := c.Clients[DefaultName]
	if got.MaxRetries != -1 || got.MinRetryBackoff != -1 {
		t.Errorf("负值应原样传给 go-redis，got=%+v", got)
	}
}

func TestConfig_退避写裸的负一启动失败(t *testing.T) {
	// 时长得带单位，文档里写的是 -1ns。裸写 -1 解不成时长，要在启动时报出来
	if err := loadErr(t, "XRedis:\n  Addr: h:6379\n  MinRetryBackoff: -1\n"); err == nil {
		t.Fatal("裸写 -1 应当解码失败")
	}
}

func TestConfig_退避交给goredis时的默认值与文档一致(t *testing.T) {
	// 注释和 docs/config.md 写的是 10ms / 1s（v9.22.0）。升级 go-redis 之后
	// 数字变了，这里先红，文档跟着改
	rc := redis.NewClient(&redis.Options{Addr: "127.0.0.1:1"})
	defer rc.Close()
	o := rc.Options()
	if o.MinRetryBackoff != 10*time.Millisecond || o.MaxRetryBackoff != time.Second {
		t.Errorf("go-redis 的默认退避变了：min=%v max=%v", o.MinRetryBackoff, o.MaxRetryBackoff)
	}
}

func TestValidate(t *testing.T) {
	ok := DefaultClientConfig()
	if err := ok.validate(); err != nil {
		t.Errorf("默认配置应当合法：%v", err)
	}

	bad := ok
	bad.Addr = ""
	if err := bad.validate(); err == nil {
		t.Error("Addr 为空应当报错")
	}

	bad = ok
	bad.DB = -1
	if err := bad.validate(); err == nil {
		t.Error("DB 为负应当报错")
	}
}
