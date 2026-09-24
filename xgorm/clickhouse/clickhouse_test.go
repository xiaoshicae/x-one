package clickhouse

import (
	"context"
	"database/sql"
	"database/sql/driver"
	"errors"
	"io"
	"net"
	"net/url"
	"slices"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"gorm.io/driver/clickhouse"
	"gorm.io/gorm"

	"github.com/xiaoshicae/x-one/xgorm"
)

func cfg(dsn string, dial time.Duration) xgorm.ClientConfig {
	c := xgorm.DefaultClientConfig()
	c.Driver, c.DSN, c.DialTimeout = Driver, dsn, dial
	return c
}

func TestRegister_import进来就注册好了(t *testing.T) {
	// 使用者只写一行匿名 import，不该再调用任何函数
	if !slices.Contains(xgorm.Drivers(), Driver) {
		t.Fatalf("匿名 import 之后 clickhouse 就该在已注册列表里，got=%v", xgorm.Drivers())
	}
}

func TestResolve_注入建连超时(t *testing.T) {
	dsn, info, err := resolve(cfg("clickhouse://u:p@h:9000/analytics", 300*time.Millisecond))
	if err != nil {
		t.Fatal(err)
	}
	q := mustQuery(t, dsn)
	if q.Get(dialTimeoutKey) != "300ms" {
		t.Errorf("建连超时该注进 DSN，got=%q", q.Get(dialTimeoutKey))
	}
	if info.Driver != "clickhouse" || info.Addr != "h:9000" || info.DB != "analytics" {
		t.Errorf("连接信息不对，got=%+v", info)
	}
}

func TestResolve_DSN里写了的不覆盖(t *testing.T) {
	// 配置里的值只是默认值，使用者显式写进 DSN 的一律以他为准
	dsn, info, err := resolve(cfg("clickhouse://h:9000/db?dial_timeout=5s", time.Millisecond))
	if err != nil {
		t.Fatal(err)
	}
	if got := mustQuery(t, dsn).Get(dialTimeoutKey); got != "5s" {
		t.Errorf("DSN 里已有的不该被覆盖，got=%q", got)
	}
	// 建连验证的预算按 info 里的超时算，DSN 里写的更长就得报这个更长的
	if info.DialTimeout != 5*time.Second {
		t.Errorf("连接信息里的建连超时该是 DSN 里写的 5s，got=%v", info.DialTimeout)
	}
}

func TestResolve_超时为零就不注入(t *testing.T) {
	dsn, _, err := resolve(cfg("clickhouse://h:9000/db", 0))
	if err != nil {
		t.Fatal(err)
	}
	if mustQuery(t, dsn).Has(dialTimeoutKey) {
		t.Errorf("没配超时就不该凭空注一个，got=%s", dsn)
	}
}

func TestResolve_不是URL的DSN直接拒绝且不回显(t *testing.T) {
	// 驱动并不接受裸的 host:port（实测 ParseDSN("10.255.255.1:9000") 报错），
	// 原样透传的话，它建连时的解析错误会连同整串 DSN、包括明文密码一起进日志。
	// scheme 拼错驱动倒是认，但会绕过这里的超时注入
	const secret = "hunter2"
	for _, raw := range []string{
		"10.255.255.1:9000",
		"u:" + secret + "@h:9000/db",
		" clickhouse://u:" + secret + "@h:9000/db", // 前面多一个空格
		"clickhous://u:" + secret + "@h:9000/db",   // scheme 拼错
	} {
		_, _, err := resolve(cfg(raw, time.Second))
		if err == nil {
			t.Errorf("%q 该被拒绝", raw)
			continue
		}
		if strings.Contains(err.Error(), secret) {
			t.Errorf("错误信息里出现了凭证：%v", err)
		}
	}
}

func TestResolve_驱动自己解析不了的也在这里拦下且不回显(t *testing.T) {
	// 留到建连时才报，驱动的原始错误就原样进日志了——
	// http_proxy 解析失败时它回显的是整个代理地址，里面可能就有凭证
	const secret = "hunter2"
	for _, raw := range []string{
		"https://u:" + secret + "@h:8443/db",                       // https 没配 secure=true，驱动拒绝
		"clickhouse://u:" + secret + "@h:9000/db?dial_timeout=abc", // 时长写错
		"clickhouse://h:9000/db?http_proxy=" + url.QueryEscape("http://u:"+secret+"@proxy:3128/%zz"),
	} {
		_, _, err := resolve(cfg(raw, time.Second))
		if err == nil {
			t.Errorf("%q 该被拒绝", raw)
			continue
		}
		if strings.Contains(err.Error(), secret) {
			t.Errorf("错误信息里出现了凭证：%v", err)
		}
	}
}

func TestResolve_各种scheme都认(t *testing.T) {
	for _, scheme := range []string{"clickhouse", "tcp", "http", "https"} {
		dsn := scheme + "://h:9000/db"
		if scheme == "https" {
			dsn += "?secure=true" // 驱动要求 https 必须同时开 secure
		}
		_, info, err := resolve(cfg(dsn, time.Second))
		if err != nil {
			t.Fatalf("%s: %v", scheme, err)
		}
		if info.Addr != "h:9000" {
			t.Errorf("%s: 该解出地址，got=%+v", scheme, info)
		}
	}
}

func TestResolve_解析失败时错误里不带DSN(t *testing.T) {
	// url.Parse 的错误里带着整串 DSN，而错误会被记下来
	const secret = "hunter2"
	_, _, err := resolve(cfg("clickhouse://u:"+secret+"@h:9000/db\x7f\x00", time.Second))
	if err == nil {
		t.Fatal("非法 DSN 应当报错")
	}
	if strings.Contains(err.Error(), secret) {
		t.Errorf("错误信息里出现了凭证：%v", err)
	}
}

func mustQuery(t *testing.T, dsn string) url.Values {
	t.Helper()
	u, err := url.Parse(dsn)
	if err != nil {
		t.Fatal(err)
	}
	return u.Query()
}

func TestResolve_query解析不了时报错而不是悄悄丢参数(t *testing.T) {
	// 理由同 xgorm 的 postgres 分支：吞掉解析错误会让密码凭空消失
	const secret = "p%ssw0rd"
	_, _, err := resolve(cfg("clickhouse://h:9000/db?password="+secret, time.Second))
	if err == nil {
		t.Fatal("解不出来的 query 应当报错，而不是悄悄丢掉那个参数")
	}
	if strings.Contains(err.Error(), secret) {
		t.Errorf("错误信息里出现了凭证：%v", err)
	}
}

func TestResolve_合法的百分号转义照常保留(t *testing.T) {
	dsn, _, err := resolve(cfg("clickhouse://h:9000/db?password=p%25ssw0rd", time.Second))
	if err != nil {
		t.Fatal(err)
	}
	if got := mustQuery(t, dsn).Get("password"); got != "p%ssw0rd" {
		t.Errorf("密码该原样留着，got=%q", got)
	}
}

func TestOpen_关掉驱动自带的那次查版本(t *testing.T) {
	// 那次查询写死了 context.Background()，就在 gorm.Open 里，重试也轮不到它
	d, ok := open("clickhouse://h:9000/db").(*clickhouse.Dialector)
	if !ok {
		t.Fatalf("该是驱动自己的 Dialector，got %T", d)
	}
	if !d.SkipInitializeWithVersion {
		t.Error("Initialize 里那次查版本必须关掉，挪到 Ready 里做")
	}
}

func TestApplyVersion_与驱动的规则一致(t *testing.T) {
	cases := []struct {
		v                   string
		noRename, noPrecise bool
	}{
		{"20.3.1.1", true, true},
		{"21.8.3.44", false, true},
		{"23.8.2.7", false, false},
		{"not-a-version", false, false},
	}
	for _, c := range cases {
		var got clickhouse.Config
		applyVersion(&got, c.v)
		if got.DontSupportRenameColumn != c.noRename || got.DontSupportColumnPrecision != c.noPrecise {
			t.Errorf("%s: got rename=%v precision=%v", c.v, got.DontSupportRenameColumn, got.DontSupportColumnPrecision)
		}
	}
}

func TestProbeVersion_查到的版本设进方言且受ctx管(t *testing.T) {
	db := fakeCH(t, "20.3.1.1")
	if err := probeVersion(context.Background(), db); err != nil {
		t.Fatal(err)
	}
	d := db.Dialector.(*clickhouse.Dialector)
	if d.Version != "20.3.1.1" || !d.DontSupportRenameColumn || !d.DontSupportColumnPrecision {
		t.Errorf("版本和两个开关该设好，got version=%q config=%+v", d.Version, *d.Config)
	}

	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := probeVersion(ctx, fakeCH(t, "23.8")); !errors.Is(err, context.Canceled) {
		t.Errorf("ctx 取消了就不该再查，got %v", err)
	}
}

func TestRegister_Ready接的是查版本(t *testing.T) {
	// 调用点：Open 关掉了驱动那次查询，Ready 没接上的话版本就永远没人查了
	if dialect.Ready == nil {
		t.Fatal("clickhouse 方言该带着 Ready")
	}
	db := fakeCH(t, "20.3.1.1")
	if err := dialect.Ready(context.Background(), db); err != nil {
		t.Fatal(err)
	}
	if !db.Dialector.(*clickhouse.Dialector).DontSupportRenameColumn {
		t.Error("注册进去的 Ready 该是 probeVersion")
	}
}

func TestNew_首次建连受ctx管也会重试(t *testing.T) {
	// 驱动在 gorm.Open 里用 context.Background() 查版本：实测对一个收下连接
	// 却不回话的地址，ctx 早已取消也要等满 dial_timeout，然后直接失败——
	// xgorm 的建连重试一次都没轮上
	addr, accepts := withSilentServer(t)
	c := cfg("clickhouse://u:p@"+addr+"/db", time.Second)

	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	start := time.Now()
	if _, _, err := xgorm.New(ctx, c); err == nil {
		t.Fatal("ctx 已取消时不该建连成功")
	}
	if elapsed := time.Since(start); elapsed > 300*time.Millisecond {
		t.Errorf("ctx 已取消就该当场放弃，实际等了 %v", elapsed)
	}

	c.DialTimeout = 50 * time.Millisecond
	accepts.Store(0)
	if _, _, err := xgorm.New(context.Background(), c); err == nil {
		t.Fatal("对方不回话时不该建连成功")
	}
	if n := accepts.Load(); n < 3 {
		t.Errorf("首次建连失败要走 xgorm 的重试，实际只连了 %d 次", n)
	}
}

// withSilentServer 起一个收下连接但永远不回话的服务，返回地址和累计连接数
func withSilentServer(t *testing.T) (string, *atomic.Int64) {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	var accepts atomic.Int64
	var conns []net.Conn
	done := make(chan struct{})
	go func() {
		defer close(done)
		for {
			c, err := ln.Accept()
			if err != nil {
				for _, c := range conns {
					c.Close()
				}
				return
			}
			accepts.Add(1)
			conns = append(conns, c)
		}
	}()
	t.Cleanup(func() { ln.Close(); <-done })
	return ln.Addr().String(), &accepts
}

// fakeCH 造一个 *gorm.DB：方言是真的 ClickHouse Dialector，连接池背后是个
// 只会回版本号的假驱动。查版本那一步要量的就是方言上的开关，不必起一个 ClickHouse
func fakeCH(t *testing.T, v string) *gorm.DB {
	t.Helper()
	pool := sql.OpenDB(verConnector{v})
	t.Cleanup(func() { pool.Close() })
	db, err := gorm.Open(clickhouse.New(clickhouse.Config{Conn: pool, SkipInitializeWithVersion: true}),
		&gorm.Config{DisableAutomaticPing: true})
	if err != nil {
		t.Fatal(err)
	}
	return db
}

type verConnector struct{ v string }

func (c verConnector) Connect(context.Context) (driver.Conn, error) { return verConn(c), nil }
func (c verConnector) Driver() driver.Driver                        { return nil }

type verConn struct{ v string }

func (c verConn) Prepare(string) (driver.Stmt, error) { return verStmt(c), nil }
func (verConn) Close() error                          { return nil }
func (verConn) Begin() (driver.Tx, error)             { return nil, errors.New("not supported") }

type verStmt struct{ v string }

func (verStmt) Close() error                                { return nil }
func (verStmt) NumInput() int                               { return 0 }
func (verStmt) Exec([]driver.Value) (driver.Result, error)  { return nil, errors.New("not supported") }
func (s verStmt) Query([]driver.Value) (driver.Rows, error) { return &verRows{v: s.v}, nil }

type verRows struct {
	v    string
	done bool
}

func (*verRows) Columns() []string { return []string{"version()"} }
func (*verRows) Close() error      { return nil }
func (r *verRows) Next(dest []driver.Value) error {
	if r.done {
		return io.EOF
	}
	r.done, dest[0] = true, r.v
	return nil
}
