package xclient

import (
	"context"
	"errors"
	"testing"
	"time"
)

var errDenied = errors.New("password authentication failed")

func policy(auth func(error) bool) ProbePolicy {
	return ProbePolicy{Attempts: 3, Timeout: 50 * time.Millisecond, Interval: time.Millisecond, AuthFailed: auth}
}

func isDenied(err error) bool { return errors.Is(err, errDenied) }

func TestProbe_成功就不再试(t *testing.T) {
	calls := 0
	err := Probe(context.Background(), policy(isDenied), func(context.Context) error {
		calls++
		if calls < 2 {
			return errors.New("connection refused")
		}
		return nil
	})
	if err != nil || calls != 2 {
		t.Errorf("第二次成功就该停，err=%v calls=%d", err, calls)
	}
}

func TestProbe_认证失败试一次就返回原错误(t *testing.T) {
	// 密码错了再试也是错，只是多等几轮退避；返回的要是 fn 的那个错误本身，
	// 调用方靠它说「认证失败」，也靠 errors.As 取驱动的错误类型
	calls := 0
	wrapped := &wrapErr{errDenied}
	err := Probe(context.Background(), policy(isDenied), func(context.Context) error {
		calls++
		return wrapped
	})
	if calls != 1 {
		t.Errorf("认证失败不该重试，试了 %d 次", calls)
	}
	if err != error(wrapped) {
		t.Errorf("返回的应是 fn 的错误本身，got=%#v", err)
	}
}

func TestProbe_认不出认证失败就照常重试(t *testing.T) {
	for name, auth := range map[string]func(error) bool{
		"没给 AuthFailed": nil,
		"AuthFailed 不认": func(error) bool { return false },
	} {
		calls := 0
		err := Probe(context.Background(), policy(auth), func(context.Context) error {
			calls++
			return errDenied
		})
		if calls != 3 || !errors.Is(err, errDenied) {
			t.Errorf("%s：应试满 3 次并返回最后的错误，calls=%d err=%v", name, calls, err)
		}
	}
}

func TestProbe_每次尝试带着自己的截止时间(t *testing.T) {
	var deadlines []time.Duration
	_ = Probe(context.Background(), policy(nil), func(ctx context.Context) error {
		d, ok := ctx.Deadline()
		if !ok {
			t.Fatal("fn 拿到的 ctx 应带着这一次的截止时间")
		}
		deadlines = append(deadlines, time.Until(d))
		return errors.New("timeout")
	})
	for _, d := range deadlines {
		if d > 50*time.Millisecond {
			t.Errorf("单次截止时间不该超过 Timeout，got=%v", d)
		}
	}
}

func TestProbe_ctx取消时不再试(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	calls := 0
	err := Probe(ctx, policy(nil), func(context.Context) error {
		calls++
		cancel()
		return errors.New("connection refused")
	})
	if calls != 1 || !errors.Is(err, context.Canceled) {
		t.Errorf("取消之后不该再试，并如实报取消，calls=%d err=%v", calls, err)
	}
}

type wrapErr struct{ err error }

func (w *wrapErr) Error() string { return "connect: " + w.err.Error() }
func (w *wrapErr) Unwrap() error { return w.err }
