package xone

import (
	"fmt"
	"io"
	"os"
	"runtime"
	"runtime/debug"
	"strings"

	"github.com/xiaoshicae/x-one/internal/config"
	"github.com/xiaoshicae/x-one/internal/hook"
)

// bannerText 启动 banner。
//
// 只在 stderr 是终端时打：在容器里、被重定向到文件或者接在日志采集器后面时，
// 一段带颜色的多行字符画就是日志平台里几行解析失败的垃圾——框架写出的每一行都该是 JSON。
var bannerText = []string{
	` __  __       ___   _   _  _____ `,
	` \ \/ /      / _ \ | \ | || ____|`,
	`  \  /_____ | | | ||  \| ||  _|  `,
	`  /  \_____|| |_| || |\  || |___ `,
	` /_/\_\      \___/ |_| \_||_____|`,
}

// bannerColors 每一行的颜色（24 位 ANSI），由浅蓝渐变到淡紫
var bannerColors = [][3]int{{110, 180, 210}, {114, 168, 209}, {120, 156, 206}, {130, 144, 200}, {142, 132, 194}}

// stderr banner 写到哪里。测试换成管道，看 Run 是不是只在终端里才打
var stderr = os.Stderr

// printBanner stderr 是终端时往 w 写 banner
func printBanner(w io.Writer, terminal bool) {
	if !terminal {
		return
	}
	var b strings.Builder
	b.WriteString("\n")
	for i, l := range bannerText {
		c := bannerColors[min(i, len(bannerColors)-1)]
		fmt.Fprintf(&b, "\x1b[38;2;%d;%d;%dm%s\x1b[0m\n", c[0], c[1], c[2], l)
	}
	fmt.Fprintf(&b, "  \x1b[38;2;110;180;210m::\x1b[0m x-one \x1b[38;2;110;180;210m::\x1b[0m  \x1b[2m%s\x1b[0m\n\n", version())
	fmt.Fprint(w, b.String())
}

// isTerminal f 是不是终端（字符设备）。不引第三方库：核心只依赖 yaml
func isTerminal(f *os.File) bool {
	st, err := f.Stat()
	return err == nil && st.Mode()&os.ModeCharDevice != 0
}

// modulePath 核心模块的路径，从二进制的构建信息里找它的版本
const modulePath = "github.com/xiaoshicae/x-one"

// version 编进这个二进制的 x-one 版本：取自构建信息，发布时不用改任何常量。
// 用 replace 指向本地目录时构建信息里没有真实版本，显示 (devel)
func version() string {
	bi, ok := debug.ReadBuildInfo()
	if !ok {
		return "(devel)"
	}
	return moduleVersion(bi)
}

func moduleVersion(bi *debug.BuildInfo) string {
	if bi.Main.Path == modulePath {
		return orDevel(bi.Main.Version)
	}
	for _, d := range bi.Deps {
		if d.Path == modulePath {
			if d.Replace != nil {
				return "(devel)"
			}
			return orDevel(d.Version)
		}
	}
	return "(devel)"
}

func orDevel(v string) string {
	if v == "" {
		return "(devel)"
	}
	return v
}

// stageNames 调试输出里档位的名字
var stageNames = map[hook.Stage]string{
	hook.StageLog:       "Log",
	hook.StageTelemetry: "Telemetry",
	hook.StageClient:    "Client",
	hook.StageBusiness:  "Business",
	hook.StageServer:    "Server",
}

// debugHooks XONE_DEBUG 开着时列出启动钩子的执行顺序；停止钩子按它倒过来
func debugHooks(entries []hook.Entry) {
	if !config.Debug() {
		return
	}
	var b strings.Builder
	fmt.Fprintf(&b, "[xone debug] x-one %s, %s\n", version(), runtime.Version())
	fmt.Fprintf(&b, "[xone debug] start hooks, in order (stop hooks run in reverse):\n")
	for i, e := range entries {
		fmt.Fprintf(&b, "  %d. %-9s %s\n", i+1, stageNames[e.Stage], e.Name)
	}
	fmt.Fprint(config.DebugOut, b.String())
}
