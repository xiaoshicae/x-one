// Package xcache 按配置装好 ristretto 本地缓存，使用者拿到的是原生的 *ristretto.Cache。
//
//	xcache.Set("user:1", u)          // 用配置里的默认 TTL 和 cost=1
//	v, ok := xcache.Get("user:1")
//	xcache.C().SetWithTTL(k, v, 8, time.Hour)  // 要完整控制就用原生的
//
// 包级的 Get/Set 只是默认实例上的便利写法，没有另造一个缓存类型——
// 那会多出一套要学、要维护、还总缺点什么的 API。
package xcache

import (
	"fmt"
	"time"

	"github.com/xiaoshicae/x-one/xconfig"
)

// ConfigKey 本模块在配置文件里的顶层 key
const ConfigKey = "XCache"

// DefaultName C() 不带参数时取的那个实例的名字
const DefaultName = "default"

// Config 本模块的配置。两种写法：
//
//	XCache:                 # 单实例
//	  MaxCost: 100000
//
//	XCache:                 # 多实例
//	  Clients:
//	    default: {MaxCost: 100000}
//	    session: {MaxCost: 10000, DefaultTTL: 30m}
type Config struct {
	// Clients 按名字组织的实例。单实例写法会被规整成一个名为 default 的实例。
	Clients map[string]ClientConfig
}

// ClientConfig 一个缓存实例的配置
type ClientConfig struct {
	// NumCounters 用于统计访问频率的计数器个数。默认 1000000。
	//
	// 建议取预期条目数的 10 倍：ristretto 靠它判断哪些键值得留下，
	// 给少了准入判断会失准，缓存命中率上不去。
	NumCounters int64 `yaml:"NumCounters"`

	// MaxCost 缓存总成本上限。默认 100000。
	//
	// 本包写入时 cost 固定为 1，所以它等价于「最多存多少条」。
	// 自己调 C().Set 传别的 cost 时，它才是真正的成本上限。
	//
	// 这里统计的是使用者给出的 cost，不含 ristretto 每条 56 字节的内部开销。
	// 按字节记 cost 时把这部分算进自己的预算里。
	MaxCost int64 `yaml:"MaxCost"`

	// BufferItems Get 的内部缓冲区大小。默认 64，官方建议值。
	BufferItems int64 `yaml:"BufferItems"`

	// DefaultTTL 包级 Set 写入时用的过期时间。默认 5m。
	//
	// 0 表示永不过期；负数启动失败——ristretto 会把 ttl<0 的写入直接丢掉。
	DefaultTTL time.Duration `yaml:"DefaultTTL"`
}

// DefaultClientConfig 单个实例的全部默认值集中在这里
func DefaultClientConfig() ClientConfig {
	return ClientConfig{
		NumCounters: 1_000_000,
		MaxCost:     100_000,
		BufferItems: 64,
		DefaultTTL:  5 * time.Minute,
	}
}

// DefaultConfig 默认没有任何实例——没配 XCache 就不建缓存
func DefaultConfig() Config { return Config{} }

// loadConfig 读配置文件里的这一块。单实例和多实例两种写法都收，
// 每个实例先铺上 DefaultClientConfig 再解，规则见 xconfig.UnmarshalClients
func loadConfig() (Config, error) {
	clients, err := xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)
	return Config{Clients: clients}, err
}

// validate 检查配置本身说不通的地方
func (c ClientConfig) validate() error {
	if c.NumCounters <= 0 {
		return fmt.Errorf("NumCounters must be > 0, got=%d", c.NumCounters)
	}
	if c.MaxCost <= 0 {
		return fmt.Errorf("MaxCost must be > 0, got=%d", c.MaxCost)
	}
	if c.BufferItems <= 0 {
		return fmt.Errorf("BufferItems must be > 0, got=%d", c.BufferItems)
	}
	// ristretto 对 ttl<0 的写入直接丢弃并返回 false（v2.4.2 SetWithTTL），
	// 配成负数的话包级 Set 一条都存不进去，而且没有任何报错
	if c.DefaultTTL < 0 {
		return fmt.Errorf("DefaultTTL must not be negative (0 means never expire), got=%s", c.DefaultTTL)
	}
	return nil
}
