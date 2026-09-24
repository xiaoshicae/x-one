package xclient

import (
	"context"
	"time"

	"github.com/xiaoshicae/x-one/xutil"
)

// ProbePolicy 启动期建连探测怎么试：试几次、每次多久、第一次退避多久、哪些错误不必再试
type ProbePolicy struct {
	// Attempts 最多试几次，<1 按 1 次算
	Attempts int

	// Timeout 单次尝试的预算
	Timeout time.Duration

	// Interval 第一次退避的上界，之后逐次翻倍、带抖动，规则见 xutil.Retry
	Interval time.Duration

	// AuthFailed 服务端是否已经明确拒绝了这组凭证。可以留空，留空就是一律重试。
	//
	// 认得出来的不再重试：密码错了再试几次也是错，只是把同一个错误多等几轮退避
	// （默认最多 3s）才报出来，还会在服务端多留几条认证失败的记录。
	AuthFailed func(error) bool
}

// Probe 按 p 反复调用 fn，直到成功、认证被拒、试满次数或 ctx 被取消。
//
// fn 收到的 ctx 带着这一次尝试的截止时间，fn 要尊重它——Probe 不会替 fn 把
// 一个卡住的调用丢下不管：sql.DB 的 Close 会等在途的查询，丢下的协程并不会
// 因为连接池关了就返回。驱动本身不理 ctx 的（go-redis 只认截止时间、不认取消）
// 由调用方自己在 fn 里处理。
//
// 返回值：
//   - 成功时 nil；
//   - 认证被拒时，就是 fn 返回的那个错误，一次都不多试；
//   - 其余情况同 xutil.Retry：最后一次的错误，ctx 被取消时还同时满足 errors.Is(err, ctx.Err())。
//
// 调用方拿 p.AuthFailed 再问一次，就能决定报「认证失败」还是「连不上」。
func Probe(ctx context.Context, p ProbePolicy, fn func(context.Context) error) error {
	return xutil.Retry(ctx, p.Attempts, p.Timeout, p.Interval, func(ctx context.Context) error {
		err := fn(ctx)
		if err != nil && p.AuthFailed != nil && p.AuthFailed(err) {
			return xutil.Permanent(err)
		}
		return err
	})
}
