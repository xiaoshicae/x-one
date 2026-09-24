package harness

import (
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"
)

// build 两个二进制一起编，整个测试进程只编一次
var build struct {
	once sync.Once
	dir  string
	out  []byte
	err  error
}

// ServiceBinary e2e/service 编出来的可执行文件
func ServiceBinary(t testing.TB) string { return binary(t, "service") }

// BaselineBinary e2e/baseline 编出来的可执行文件
func BaselineBinary(t testing.TB) string { return binary(t, "baseline") }

func binary(t testing.TB, name string) string {
	t.Helper()
	Require(t)
	build.once.Do(func() {
		build.dir, build.err = os.MkdirTemp("", "xone-e2e-bin-")
		if build.err != nil {
			return
		}
		// 和 scripts/test.sh 一样 GOWORK=off：测的是 e2e/go.mod 自己解出来的依赖。
		// 不加 -race：压测要的是生产形态的数字
		cmd := exec.Command("go", "build", "-o", build.dir+string(filepath.Separator), "./service", "./baseline")
		cmd.Dir = ModuleDir()
		cmd.Env = append(os.Environ(), "GOWORK=off")
		build.out, build.err = cmd.CombinedOutput()
	})
	if build.err != nil {
		t.Fatalf("build e2e binaries: %v\n%s", build.err, build.out)
	}
	return filepath.Join(build.dir, name)
}

func removeBinaries() {
	if build.dir != "" {
		os.RemoveAll(build.dir)
	}
}
