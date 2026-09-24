package e2e

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"

	"github.com/xiaoshicae/x-one/e2e/harness"
)

// chResult GET /ch/sleep、/ch/echo、/ch/stream 的结果：和 faultDepResult 一样量两段耗时，另带 body 里的 got / rows
type chResult struct {
	faultDepResult
	Got  string
	Rows int
}

// chCall 调一个 /ch/... 接口，客户端最多等 wait。不带 t，哪个协程里都能调
func chCall(p *harness.Process, path string, wait time.Duration) chResult {
	ctx, cancel := context.WithTimeout(context.Background(), wait)
	defer cancel()
	start := time.Now()
	r, err := p.Request(ctx, http.MethodGet, path, nil)
	res := chResult{faultDepResult: faultDepResult{Status: r.Status, Took: time.Since(start), ReqErr: err}}
	if err == nil {
		var body struct {
			ElapsedMS float64 `json:"elapsed_ms"`
			Error     string  `json:"error"`
			Got       string  `json:"got"`
			Rows      int     `json:"rows"`
		}
		_ = json.Unmarshal(r.Body, &body)
		res.Server = time.Duration(body.ElapsedMS * float64(time.Millisecond))
		res.Error, res.Got, res.Rows = body.Error, body.Got, body.Rows
	}
	return res
}

// chFillPool 并发跑 n 条 /ch/sleep，把池子撑到 n 条连接：之后卡住时，前 n 条查询各用一条池里现成的连接
func chFillPool(t *testing.T, p *harness.Process, n int) {
	t.Helper()
	var wg sync.WaitGroup
	res := make([]chResult, n)
	for i := range n {
		wg.Add(1)
		go func() { defer wg.Done(); res[i] = chCall(p, "/ch/sleep?ms=200", 10*time.Second) }()
	}
	wg.Wait()
	for _, r := range res {
		if r.Status != http.StatusOK {
			t.Fatalf("故障前 SELECT sleep 应 200，实际 %v", r.faultDepResult)
		}
	}
}

// 运行中 ClickHouse 进程挂了（端口拒绝连接）：
//
//   - 用到 ClickHouse 的操作当场报错（新连接被拒不需要等任何超时），错误里有地址；
//   - 另两个实例（PG、MySQL）和不碰 ClickHouse 的接口不受影响：多实例各是各的连接池；
//   - ClickHouse 回来之后不用重启就恢复
func TestClickHouse_运行中ClickHouse拒绝连接_用到它的操作当场报错_其他实例不受影响_恢复后自动恢复(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	chp := harness.NewProxy(t, harness.CHAddr())
	p := chStart(t, harness.Options{CHAddr: chp.Addr()})
	ids, _ := chInsert(t, p, "before-cut", 1)

	chp.Cut()

	t.Run("用到ClickHouse的操作当场报错", func(t *testing.T) {
		var took []time.Duration
		for i := range 5 {
			r := chDep(p, "", 30*time.Second)
			if r.ReqErr != nil || r.Status != http.StatusServiceUnavailable {
				t.Fatalf("第 %d 次：ClickHouse 拒绝连接时 /dep?target=ch 应回 503，实际 %v", i+1, r)
			}
			if !strings.Contains(r.Error, "connection refused") || !strings.Contains(r.Error, chp.Addr()) {
				t.Errorf("第 %d 次：错误应说清连不上哪（%s，connection refused），实际 %q", i+1, chp.Addr(), r.Error)
			}
			if r.Server > chDialTimeout {
				t.Errorf("第 %d 次：新连接被拒不该等任何超时，实际 %v", i+1, r.Server)
			}
			took = append(took, r.Server)
		}
		t.Logf("数字：ClickHouse 拒绝连接时 SELECT 1 依次用了 %v；%s", took, faultSummary(took))
		for _, c := range []struct {
			method, path string
			body         any
		}{
			{http.MethodGet, fmt.Sprintf("/ch/events/%d", ids[0]), nil},
			{http.MethodGet, "/ch/stats?name=x", nil},
			{http.MethodPost, "/ch/events", map[string]any{"name": "while-cut", "values": []int{1}}},
		} {
			r, took := faultTimed(t, p, c.method, c.path, c.body)
			if r.Status != http.StatusServiceUnavailable {
				t.Errorf("ClickHouse 不可用时 %s %s 应 503，实际 %v", c.method, c.path, r)
			}
			if took > chDialTimeout {
				t.Errorf("ClickHouse 拒绝连接时 %s %s 应当场报错，实际用了 %v", c.method, c.path, took)
			}
		}
	})

	t.Run("PG和MySQL和不碰ClickHouse的接口不受影响", func(t *testing.T) {
		faultQuick(t, p, "GET /ping", "/ping", http.StatusOK, 200*time.Millisecond)
		faultQuick(t, p, "GET /dep?target=db", "/dep?target=db", http.StatusOK, 200*time.Millisecond)
		faultQuick(t, p, "GET /dep?target=mysql", "/dep?target=mysql", http.StatusOK, 200*time.Millisecond)
	})

	t.Run("ClickHouse回来之后自动恢复", func(t *testing.T) {
		chp.Restore()
		took, n := chWaitDep(t, p, 10*time.Second)
		t.Logf("数字：ClickHouse 恢复监听之后 %s（第 %d 次探测）SELECT 1 回到 200", faultMS(took), n)
		fresh, _ := chInsert(t, p, "after", 2)
		if r := p.Get(t, fmt.Sprintf("/ch/events/%d", fresh[0])); r.Status != http.StatusOK {
			t.Errorf("恢复后读写应照常，实际 %v", r)
		}
	})
}

// 运行中 ClickHouse 不回话 / 主机宕机，调用方给了 200ms 的截止时间。
//
// docs/config.md「ClickHouse 的超时与取消」：「每条查询开始时设一次读 deadline；调用方的 ctx 有截止时间时改用它」。
// 这一条只对池里现成的连接成立。新建连接走的是 database/sql 的 driver.Open（clickhouse-go v2.30.0 的 stdDriver
// 没有实现 driver.DriverContext，database/sql 拿不到 ctx 传给它）：拨号是 net.DialTimeout、握手按 dial_timeout
// 设整条连接的 deadline，都不看调用方的 ctx。所以卡住之后先是几条 200ms 返回的（池里的连接，被取消之后关掉），
// 池子耗光之后每条都要等满 dial_timeout（默认 500ms）才返回——比调用方给的多出 300ms，但有上界
func TestClickHouse_卡住或宕机时_池里的连接听调用方的截止时间_新建连接要等满dial_timeout(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	const (
		deadline = 200 * time.Millisecond
		pooled   = 3
	)
	for _, m := range faultSilentModes {
		t.Run(m.name, func(t *testing.T) {
			t.Parallel()
			chp := harness.NewProxy(t, harness.CHAddr())
			p := chStart(t, harness.Options{CHAddr: chp.Addr()})
			chFillPool(t, p, pooled)
			m.inject(chp)
			var fromPool, fresh []time.Duration
			for i := range pooled + 4 {
				accepted := chp.Accepted()
				r := chDep(p, deadline.String(), 10*time.Second)
				if r.ReqErr != nil || r.Status != http.StatusServiceUnavailable {
					t.Fatalf("第 %d 次：卡住时应以 503 返回，实际 %v", i+1, r)
				}
				// 新建的连接：对端不回话时代理接受了新连接，主机宕机时报的是 dial tcp
				isNew := chp.Accepted() > accepted || strings.Contains(r.Error, "dial tcp")
				if !isNew {
					fromPool = append(fromPool, r.Server)
					if r.Server > deadline+faultSlack || r.Server < deadline-10*time.Millisecond {
						t.Errorf("第 %d 次（池里的连接）：调用方给了 %v，查询应在那一刻返回，实际 %v：%s", i+1, deadline, r.Server, r.Error)
					}
					continue
				}
				fresh = append(fresh, r.Server)
				if r.Server > chDialTimeout+faultSlack {
					t.Errorf("第 %d 次（新建连接）：拨号和握手受 dial_timeout（%v）管，应在那时返回，实际 %v：%s", i+1, chDialTimeout, r.Server, r.Error)
				}
			}
			if len(fromPool) == 0 || len(fresh) == 0 {
				t.Fatalf("应当两段都测到：池里的连接 %d 次、新建连接 %d 次", len(fromPool), len(fresh))
			}
			t.Logf("数字：ClickHouse %s、调用方给 %v：池里的连接 %s；新建连接 %s", m.name, deadline, faultSummary(fromPool), faultSummary(fresh))
		})
	}
}

// 运行中 ClickHouse 不回话、调用方不给截止时间：
//
//   - 管住这一条查询的只有驱动的 read_timeout：DSN 里写了 read_timeout=2s 就在 2s 失败；
//   - 没写时它是 300s（docs/config.md：「read_timeout 没写时是 300s……后台任务不给截止时间的话，
//     对端不回话要等 5 分钟」）。等 5 分钟不现实：这里只确认 8s 后查询还挂着（实测 300.0s 才返回
//     read: read tcp …: i/o timeout，量的时候单独跑了一次）；
//   - 同一时刻 PG、MySQL 照常，而且快
func TestClickHouse_运行中ClickHouse不回话_不给截止时间只有read_timeout管_没写时是300s(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	t.Run("DSN里写了read_timeout", func(t *testing.T) {
		t.Parallel()
		const readTimeout = 2 * time.Second
		chp := harness.NewProxy(t, harness.CHAddr())
		p := chStart(t, harness.Options{Env: map[string]string{"E2E_CH_DSN": harness.CHDSN(chp.Addr()) + "?read_timeout=2s"}})
		chFillPool(t, p, 4)
		chp.SetDelay(time.Hour)

		var pg []faultDepResult
		var wg sync.WaitGroup
		wg.Add(1)
		go func() { // ClickHouse 那一批卡着的同时，PG 和 MySQL 每 100ms 探一次
			defer wg.Done()
			for i := range 20 {
				pg = append(pg, faultDep(p, []string{"db", "mysql"}[i%2], "", 5*time.Second))
				time.Sleep(100 * time.Millisecond)
			}
		}()
		res := faultBurst(p, 4, "ch", readTimeout+10*time.Second)
		wg.Wait()
		var took []time.Duration
		for i, r := range res {
			if r.ReqErr != nil || r.Status != http.StatusServiceUnavailable || !strings.Contains(r.Error, "i/o timeout") {
				t.Errorf("第 %d 个：对端不回话时查询应在 read_timeout 失败（503，i/o timeout），实际 %v", i+1, r)
				continue
			}
			if r.Server < readTimeout-20*time.Millisecond || r.Server > readTimeout+faultSlack {
				t.Errorf("第 %d 个：DSN 里写了 read_timeout=%v，查询应在那时失败，实际 %v", i+1, readTimeout, r.Server)
			}
			took = append(took, r.Server)
		}
		t.Logf("数字：ClickHouse 不回话、并发 4、read_timeout=2s：%s；错误：%s", faultSummary(took), res[0].Error)
		var other []time.Duration
		for i, r := range pg {
			if r.Status != http.StatusOK || r.Server > 200*time.Millisecond {
				t.Errorf("第 %d 次：ClickHouse 卡住时 PG / MySQL 应照常而且快，实际 %v", i+1, r)
			}
			other = append(other, r.Server)
		}
		t.Logf("数字：同一时刻 PG / MySQL 的 SELECT 1：%s", faultSummary(other))
	})

	t.Run("没写read_timeout", func(t *testing.T) {
		t.Parallel()
		const giveUp = 8 * time.Second
		chp := harness.NewProxy(t, harness.CHAddr())
		p := chStart(t, harness.Options{CHAddr: chp.Addr()})
		chp.SetDelay(time.Hour)
		r := chDep(p, "", giveUp)
		if r.ReqErr == nil {
			t.Fatalf("文档说 read_timeout 默认 300s：对端不回话、不给截止时间，%v 内查询不该返回，实际 %v", giveUp, r)
		}
		t.Logf("数字：ClickHouse 不回话、不给截止时间、没写 read_timeout：%v 后查询仍没返回（驱动默认 read_timeout %v）", giveUp, chReadTimeout)
	})
}

// 表头之后才卡住。clickhouse-go v2.30.0 的 query 在收到第一个数据块（表头）之后就把读 deadline 清掉了
// （conn_query.go：SetReadDeadline 之后 defer SetReadDeadline(time.Time{})，函数在 firstBlock 之后返回），
// 余下的数据块在另一个协程里读，不再有读超时——read_timeout 只管到表头为止。
//
// ClickHouse 对一条查询几乎立刻就回表头（实测 SELECT sleep(2) 配 read_timeout=1s 照样 2.0s 成功），
// 所以对一条在服务端跑着的查询，read_timeout 等于没有：流到一半对端不回话、调用方又没给截止时间，
// 查询就一直挂着，比 300s 还长。调用方给的截止时间和取消照样管用（process 里 select ctx.Done()）。
// docs/config.md「ClickHouse 的超时与取消」按量出来的写了这一条
func TestClickHouse_表头之后才卡住_read_timeout不再管_只有调用方的截止时间管(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	const (
		readTimeout = time.Second
		stallAt     = 700 * time.Millisecond
	)
	for _, c := range []struct {
		name     string
		timeout  time.Duration // 0 表示不给截止时间
		giveUp   time.Duration
		returned bool
	}{
		{"不给截止时间", 0, 6 * time.Second, false},
		{"调用方给了3s", 3 * time.Second, 10 * time.Second, true},
	} {
		t.Run(c.name, func(t *testing.T) {
			t.Parallel()
			chp := harness.NewProxy(t, harness.CHAddr())
			p := chStart(t, harness.Options{Env: map[string]string{"E2E_CH_DSN": harness.CHDSN(chp.Addr()) + "?read_timeout=1s"}})
			// 50 行、每行之间 100ms：健康时 5s 读完。700ms 时卡住，那时已经读到了几行
			path := "/ch/stream?rows=50&ms=100"
			if c.timeout > 0 {
				path += "&timeout=" + c.timeout.String()
			}
			time.AfterFunc(stallAt, func() { chp.SetDelay(time.Hour) })
			r := chCall(p, path, c.giveUp)
			switch {
			case !c.returned && r.ReqErr == nil:
				t.Errorf("表头之后卡住、不给截止时间：实测查询一直挂着，%v 内不该返回，实际 %v（read_timeout 又管用了？文档要跟着改）", c.giveUp, r.faultDepResult)
			case c.returned && (r.ReqErr != nil || r.Status != http.StatusServiceUnavailable):
				t.Fatalf("调用方给了 %v，查询应在那一刻失败返回 503，实际 %v", c.timeout, r.faultDepResult)
			case c.returned && (r.Server < c.timeout-20*time.Millisecond || r.Server > c.timeout+faultSlack):
				t.Errorf("调用方给了 %v，查询应在那一刻返回，实际 %v：%s", c.timeout, r.Server, r.Error)
			}
			if c.returned {
				t.Logf("数字：表头之后卡住、read_timeout=%v、调用方给 %v：%s 返回：%s", readTimeout, c.timeout, faultMS(r.Server), r.Error)
			} else {
				t.Logf("数字：表头之后卡住、read_timeout=%v、不给截止时间：%v 后查询仍挂着", readTimeout, c.giveUp)
			}
		})
	}
}

// 读超时之后那条连接还回了池里，下一条查询读到的是上一条的结果。
//
// clickhouse-go v2.30.0 的 stdDriver 靠 isConnBrokenError 判断连接坏没坏，它用的是类型断言 err.(*net.OpError)，
// 而读超时的错误被 ch-go 包了一层（read: read tcp …: i/o timeout），断言落空：QueryContext 不报 driver.ErrBadConn，
// database/sql 把这条连接还回池里。服务端那条查询却还在跑，结果晚一点照样发回来——下一条借到这条连接的查询
// 先读到的就是它。实测（read_timeout=1s、池子 1 条连接）：
//
//	A  SELECT concat('A', '/', toString((SELECT sleep(1.5))))  1.0s 失败：read: read tcp …: i/o timeout
//	B  SELECT concat('B', '/', toString((SELECT sleep(0))))    0.5s 成功，返回 "A/0"
//
// 一条查询静默地拿到了另一条的结果，没有任何报错。上游在 v2.47.0 修了（#1869 evict connections on wrapped
// net.OpErrors，改成 errors.As）；xgorm/clickhouse 钉着 v2.30.0，要修就得升驱动，模块图和文档里量过的数字都要重量
func TestClickHouse_读超时之后连接还回池里_下一条查询读到上一条的结果(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	p := chStart(t, harness.Options{
		Overlay: chOverlay("MaxOpenConns: 1\nMaxIdleConns: 1"),
		Env:     map[string]string{"E2E_CH_DSN": harness.CHDSN(harness.CHAddr()) + "?read_timeout=1s"},
	})
	a := chCall(p, "/ch/echo?tag=A&ms=1500", 10*time.Second)
	if a.ReqErr != nil || a.Status != http.StatusServiceUnavailable || !strings.Contains(a.Error, "i/o timeout") {
		t.Fatalf("A 在服务端睡 1.5s、read_timeout=1s：应在 1s 以读超时失败，实际 %v", a.faultDepResult)
	}
	b := chCall(p, "/ch/echo?tag=B", 10*time.Second)
	t.Logf("数字：A %s 失败（%s）；紧接着 B 用了 %s，status=%d got=%q error=%q", faultMS(a.Server), a.Error, faultMS(b.Server), b.Status, b.Got, b.Error)
	if b.Status == http.StatusOK && b.Got != "B/0" {
		harness.KnownBug(t, fmt.Sprintf("读超时的连接回到了池里，B（SELECT 'B'…）拿到的是 A 的结果 %q", b.Got))
	}
	if b.Status != http.StatusOK || b.Got != "B/0" {
		t.Errorf("B 应拿到自己的结果 \"B/0\"，实际 %v got=%q", b.faultDepResult, b.Got)
	}
}

// 启动时 ClickHouse 不可达，三种不可达：拒绝连接 / 对端不回话 / 主机宕机。
//
// docs/config.md：「建连重试：连不上时按 3 次重试」「其余驱动是 2 × DialTimeout」，所以启动最多等
// 3 × 1s + 3s = 6s（chStartBudget）；错误要说清是哪个实例、哪个地址，不能有密码和 DSN。
//
// 「对端不回话」那一种数得到尝试次数，它守的是「其它驱动」那一节：驱动在 Initialize 里查版本那一次
// 挪进了建连探测、跟着重试——没挪的话它发生在 gorm.Open 里，只试 1 次就失败
func TestClickHouse_启动时ClickHouse不可达_在文档的预算内失败_错误里有实例名和地址没有密码(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	pw := harness.CHPassword()
	for _, c := range []struct {
		name   string
		inject func(*harness.Proxy)
		reason string
		count  bool
	}{
		{"拒绝连接", func(p *harness.Proxy) { p.Cut() }, "connection refused", false},
		{"对端不回话", func(p *harness.Proxy) { p.SetDelay(time.Hour) }, "i/o timeout", true},
		{"主机宕机", func(p *harness.Proxy) { p.Blackhole() }, "dial tcp", false},
	} {
		t.Run(c.name, func(t *testing.T) {
			t.Parallel()
			chp := harness.NewProxy(t, harness.CHAddr())
			c.inject(chp)
			exit, p := faultStartFails(t, harness.Options{CHAddr: chp.Addr()})
			stderr := p.Stderr()
			faultMustContain(t, "stderr", stderr, "xgorm connect failed", `instance "ch"`, "cannot reach "+chp.Addr(), c.reason)
			mustNotContain(t, "进程的 stdout / stderr", p.Output(), pw, harness.CHDSN(chp.Addr()))
			if exit.Uptime > chStartBudget+faultStartSlack {
				t.Errorf("按文档，ClickHouse 建连验证最多 %d 次 × %v + 退避上界 %v = %v，实际 %v 才失败退出",
					faultPingAttempts, chAttempt, faultPingBackoffs, chStartBudget, exit.Uptime)
			}
			if n := len(p.FindLogs(func(l harness.Log) bool { return l.Msg() == "xgorm ready" })); n != 0 {
				t.Errorf("ch 实例建不起来时 xgorm 不该报 ready，实际 %d 条", n)
			}
			if c.count && chp.Accepted() != faultPingAttempts {
				t.Errorf("文档说连不上时按 %d 次重试（查版本那一次挪进了探测）；实际只试了 %d 次（错误：%s）", faultPingAttempts, chp.Accepted(), lastLine(stderr))
			}
			t.Logf("数字：启动时 ClickHouse %s：%v 后以 %d 退出（预算 %v）；代理收到 %d 个连接；错误：%s",
				c.name, exit.Uptime.Round(time.Millisecond), exit.Code, chStartBudget, chp.Accepted(), lastLine(stderr))
		})
	}
}

// 启动时 ClickHouse 不回话 / 主机宕机，这时收到 SIGTERM。
//
// MySQL、PG 的建连在信号之后几毫秒就放弃；ClickHouse 做不到：docs/config.md「ClickHouse 的超时与取消」——
// 「建连用 net.DialTimeout，不看 ctx，只受 dial_timeout 管；握手阶段同样按 dial_timeout 设整条连接的 deadline」，
// 「Ping 只认截止时间、不认取消」。xgorm 的探测是在当前协程里等的（sql.DB 的 Close 会等在途的查询，丢给别的协程
// 也关不掉它），所以信号之后要等这一次拨号或握手撞上 dial_timeout 才退出：上界是 dial_timeout（默认 500ms），
// 不会卡满整轮重试
func TestClickHouse_启动时ClickHouse不回话期间收到SIGTERM_最多再等一个dial_timeout就退出(t *testing.T) {
	harness.RequireCH(t)
	t.Parallel()
	for _, m := range faultSilentModes {
		t.Run(m.name, func(t *testing.T) {
			t.Parallel()
			chp := harness.NewProxy(t, harness.CHAddr())
			m.inject(chp)
			p := chStart(t, harness.Options{CHAddr: chp.Addr(), NoWait: true, Env: map[string]string{"E2E_STOP_TIMEOUT": "5s"}})
			p.WaitLog(t, 15*time.Second, func(l harness.Log) bool { return l.Msg() == "starting" && l.Str("hook") == "xgorm.initXGorm" })
			time.Sleep(100 * time.Millisecond) // 让第一次尝试卡进拨号或握手里
			p.Signal(syscall.SIGTERM)
			exit, ok := p.Wait(15 * time.Second)
			if !ok {
				t.Fatalf("启动期间收到 SIGTERM，15s 之后进程还活着\n%s", lastLines(p.Output(), 30))
			}
			if ls := p.FindLogs(func(l harness.Log) bool { return l.Msg() == "xgin listening" }); len(ls) > 0 {
				t.Errorf("启动期间收到信号服务不该再启动，实际日志里有 %s", ls[0].Line)
			}
			if exit.SinceSignal > chDialTimeout+faultSlack {
				t.Errorf("拨号和握手的上界是 dial_timeout（%v），信号之后最多等这么久，实际 %v", chDialTimeout, exit.SinceSignal.Round(time.Millisecond))
			}
			t.Logf("数字：启动时 ClickHouse %s、信号之后 %v 退出（dial_timeout %v、整轮预算 %v）；stderr 最后一行：%s",
				m.name, exit.SinceSignal.Round(time.Millisecond), chDialTimeout, chStartBudget, lastLine(p.Stderr()))
		})
	}
}
