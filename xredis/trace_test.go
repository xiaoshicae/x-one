package xredis

import (
	"context"
	"strings"
	"sync"
	"testing"

	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/trace"
	"go.opentelemetry.io/otel/trace/noop"
)

// recordingTP 只记下每个 Span 开始时带的属性。
//
// 不用 otel 的 sdk + tracetest：测试依赖会被 go mod tidy 记成直接依赖，
// 跟着进每个使用者的模块图（理由同 fakeRedis）。redisotel 的属性都是在
// Start 时一次给齐的，记这一处就够了。
type recordingTP struct {
	noop.TracerProvider
	mu    sync.Mutex
	attrs []attribute.KeyValue
}

func (p *recordingTP) Tracer(string, ...trace.TracerOption) trace.Tracer {
	return recordingTracer{p: p}
}

func (p *recordingTP) all() []attribute.KeyValue {
	p.mu.Lock()
	defer p.mu.Unlock()
	return append([]attribute.KeyValue(nil), p.attrs...)
}

type recordingTracer struct {
	noop.Tracer
	p *recordingTP
}

func (t recordingTracer) Start(ctx context.Context, name string, opts ...trace.SpanStartOption) (context.Context, trace.Span) {
	cfg := trace.NewSpanStartConfig(opts...)
	attrs := cfg.Attributes()
	t.p.mu.Lock()
	t.p.attrs = append(t.p.attrs, attrs...)
	t.p.mu.Unlock()
	return t.Tracer.Start(ctx, name, opts...)
}

func TestTrace_命令参数不进Span(t *testing.T) {
	// redisotel v9.22.0 默认 dbStmtEnabled=true，把整条命令连同参数写进
	// db.statement：SET 的值、AUTH 的密码，统统进了链路后端。
	// 与 xgorm 的「不记 Statement.Vars」是同一条原则
	tp := &recordingTP{}
	old := otel.GetTracerProvider()
	otel.SetTracerProvider(tp)
	t.Cleanup(func() { otel.SetTracerProvider(old) })

	f := newFakeRedis(t)
	client, closer, err := New(context.Background(), liveCfg(f))
	if err != nil {
		t.Fatal(err)
	}
	defer closer.Close()

	const secret = "s3cr3t-value"
	if err := client.Set(context.Background(), "session:1", secret, 0).Err(); err != nil {
		t.Fatal(err)
	}

	var instrumented bool
	for _, a := range tp.all() {
		if strings.Contains(a.Value.Emit(), secret) {
			t.Errorf("命令参数进了 Span：%s=%s", a.Key, a.Value.Emit())
		}
		if a.Key == "db.system" {
			instrumented = true
		}
	}
	if !instrumented {
		t.Fatal("前提：链路钩子该挂上，否则这条测试什么都没验")
	}
}
