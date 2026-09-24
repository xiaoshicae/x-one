package harness

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"sync"
)

// Log 一行 JSON 日志
type Log struct {
	// Stream 从哪来：stdout / stderr / file
	Stream string
	// Line 原文
	Line string
	// Fields 解出来的 JSON 对象
	Fields map[string]any
}

// Get 按点分路径取字段，比如 "request_headers.Authorization"。
// 先按整个名字找，找不到才按点拆：字段名里本身带点的也取得到
func (l Log) Get(path string) (any, bool) {
	var cur any = l.Fields
	for path != "" {
		m, ok := cur.(map[string]any)
		if !ok {
			return nil, false
		}
		if v, ok := m[path]; ok {
			return v, true
		}
		head, rest, found := strings.Cut(path, ".")
		if !found {
			return nil, false
		}
		if cur, ok = m[head]; !ok {
			return nil, false
		}
		path = rest
	}
	return cur, true
}

// Str 取字段并转成字符串，取不到时返回空串。数字按 fmt.Sprint 渲染（200 而不是 200.0）
func (l Log) Str(path string) string {
	v, ok := l.Get(path)
	if !ok || v == nil {
		return ""
	}
	if s, ok := v.(string); ok {
		return s
	}
	return fmt.Sprint(v)
}

// Msg 日志的 msg 字段
func (l Log) Msg() string { return l.Str("msg") }

// Level 日志的 level 字段，比如 INFO
func (l Log) Level() string { return l.Str("level") }

// parseLog 不是 JSON 对象的行返回 false
func parseLog(stream, line string) (Log, bool) {
	s := strings.TrimSpace(line)
	if !strings.HasPrefix(s, "{") {
		return Log{}, false
	}
	var m map[string]any
	if err := json.Unmarshal([]byte(s), &m); err != nil {
		return Log{}, false
	}
	return Log{Stream: stream, Line: line, Fields: m}, true
}

// ReadLogFile 读一个 JSON 行日志文件，非 JSON 的行跳过。文件不存在时返回空
func ReadLogFile(path string) ([]Log, error) {
	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	var out []Log
	sc := bufio.NewScanner(bytes.NewReader(data))
	sc.Buffer(make([]byte, 0, 64*1024), 16<<20)
	for sc.Scan() {
		if l, ok := parseLog("file", sc.Text()); ok {
			out = append(out, l)
		}
	}
	return out, sc.Err()
}

// output 收一个进程的 stdout / stderr，按行存
type output struct {
	mu     sync.Mutex
	lines  []outLine
	parsed []Log // lines 里是 JSON 的那些，按到达顺序；增量解析
	next   int   // lines 里解析到哪了
	wake   chan struct{}
}

type outLine struct {
	stream string
	text   string
}

func newOutput() *output { return &output{wake: make(chan struct{})} }

// writer 一个流的写入端。exec 为每个流单开一个协程拷贝，所以 partial 不用加锁
func (o *output) writer(stream string) *lineWriter { return &lineWriter{o: o, stream: stream} }

func (o *output) add(stream, text string) {
	o.mu.Lock()
	o.lines = append(o.lines, outLine{stream, text})
	close(o.wake)
	o.wake = make(chan struct{})
	o.mu.Unlock()
}

// changed 有新行时关闭的 channel
func (o *output) changed() <-chan struct{} {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.wake
}

func (o *output) text(stream string) string {
	o.mu.Lock()
	defer o.mu.Unlock()
	var b strings.Builder
	for _, l := range o.lines {
		if stream == "" || l.stream == stream {
			b.WriteString(l.text)
			b.WriteByte('\n')
		}
	}
	return b.String()
}

// tail 最后 n 行，两个流混在一起按到达顺序
func (o *output) tail(n int) string {
	o.mu.Lock()
	defer o.mu.Unlock()
	start := max(0, len(o.lines)-n)
	var b strings.Builder
	for _, l := range o.lines[start:] {
		fmt.Fprintf(&b, "[%s] %s\n", l.stream, l.text)
	}
	return b.String()
}

func (o *output) logs() []Log {
	o.mu.Lock()
	defer o.mu.Unlock()
	for ; o.next < len(o.lines); o.next++ {
		if l, ok := parseLog(o.lines[o.next].stream, o.lines[o.next].text); ok {
			o.parsed = append(o.parsed, l)
		}
	}
	return append([]Log(nil), o.parsed...)
}

type lineWriter struct {
	o       *output
	stream  string
	partial []byte
}

func (w *lineWriter) Write(b []byte) (int, error) {
	n := len(b)
	for {
		i := bytes.IndexByte(b, '\n')
		if i < 0 {
			w.partial = append(w.partial, b...)
			return n, nil
		}
		line := string(append(w.partial, b[:i]...))
		w.partial = w.partial[:0]
		w.o.add(w.stream, line)
		b = b[i+1:]
	}
}

// flush 进程退出后把没有换行符结尾的最后一截也收进来
func (w *lineWriter) flush() {
	if len(w.partial) > 0 {
		w.o.add(w.stream, string(w.partial))
		w.partial = nil
	}
}
