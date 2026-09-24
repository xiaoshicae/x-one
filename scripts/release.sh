#!/bin/sh
# 打 tag 发布。多模块仓库的每个 module 有自己的 tag。
#
#   scripts/release.sh v0.1.0           # 只打印要做什么，不改任何东西
#   scripts/release.sh v0.1.0 --apply   # 真的改 go.mod、提交、打 tag，再提交一次把 replace 还原（仍不推送）
#   scripts/release.sh v0.1.0 --verify  # 推送之后：用一个全新的外部工程验证装得上、跑得起来
#
# 发布前要一轮绿的 e2e（真实的 PG / MySQL / Redis）：默认在第 2 步跑 scripts/e2e.sh；
# 这个提交刚在别处跑绿过（比如 CI 的 e2e 工作流）的话加 --e2e-passed 跳过，由你担保。
#
# 推送是单独一步，由人来做：Go 的 module proxy 会永久缓存 tag，
# 推错了删不掉，只能再发一个版本盖过去。
set -e
cd "$(dirname "$0")/.."

usage() { echo "用法：scripts/release.sh v0.1.0 [--apply | --verify] [--e2e-passed]"; exit 1; }
VERSION="$1"
[ -n "$VERSION" ] || usage
shift
MODE=
E2E_PASSED=
for a; do
  case "$a" in
    --apply|--verify) [ -z "$MODE" ] || usage; MODE=$a;;
    --e2e-passed) E2E_PASSED=1;;
    *) usage;;
  esac
done
case "$VERSION" in
  # 只收 v0 / v1：模块路径里没有 /v2 后缀，go mod edit 会拒收 v2 及以上的版本号，
  # 而那一步发生在第 3 步中途——go.mod 已经改了一半，脚本才退出
  v0.*.*|v1.*.*) ;;
  v*.*.*) echo "模块路径没有 /vN 后缀，只能发 v0 / v1，got=$VERSION"; exit 1;;
  *) echo "版本号格式应为 vX.Y.Z，got=$VERSION"; exit 1;;
esac

MOD=github.com/xiaoshicae/x-one

# 要发布的子模块：签入的每个子目录 go.mod，除了这几个（根模块另打不带前缀的 tag）：
#   example             示例不是库，replace 要一直留着，这样它永远编译的是
#                       仓库当前的代码，而不是某个已发布版本
#   e2e                 真实 Web 服务测试，理由同 example
#   internal/schemagen  仓库自己的构建工具，不给使用者 import
#
# 从前这里是一张手写的「发布顺序」表，可所有 tag 都打在同一个提交上、一次推送，
# 谁先谁后根本无所谓；手写的表倒是新加一个模块就得记得改
MODS=$(git ls-files '*/go.mod' | xargs -n1 dirname | grep -vxE 'example|e2e|internal/schemagen')

# ---- --verify：发布出去的版本装得上、跑得起来 ----
# 必须在 git push --tags 之后跑。CI 跑的是工作区里的代码，模块之间还靠
# replace 互指，证明不了一个真正的使用者 go get 之后会发生什么——
# require 的版本对不对、tag 打全了没有、去掉 replace 还编不编得过，
# 全都只有这一步能回答。
verify() {
# 和发布的是同一份子模块列表，新加的模块自动进来
PKGS=$(printf "$MOD/%s\n" $MODS)
DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT

echo "== 在 $DIR 里建一个干净的消费者工程 =="
cd "$DIR"
cat > go.mod <<MOD_EOF
module verify

go 1.25
MOD_EOF

# 一个真实使用者会写的样子：起服务、连库、打点，全部走公开 API。
#
# 下面 go get 的每个模块都必须在这里 import 到：go mod tidy 会把没人 import 的
# require 删掉，那个模块就从头到尾没被编译过——xcache 和 xginswagger 从前就是
# 这样，「装得上、编得过」对它们俩什么都没证明。所以 import 由同一份列表生成，
# 每个模块一行 _ import；xgin / xmetric 另有一行具名的，同一个包两种写法 Go 是允许的
{
  cat <<'GO_EOF'
package main

import (
	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one"
	"github.com/xiaoshicae/x-one/xgin"
	"github.com/xiaoshicae/x-one/xmetric"
GO_EOF
  printf '\t_ "%s"\n' $PKGS
  cat <<'GO_EOF'
)

func main() {
	xone.MustRun(xgin.New().WithRoutes(func(e *gin.Engine) {
		e.GET("/ping", func(c *gin.Context) {
			xmetric.CounterInc("ping_total")
			c.String(200, "pong")
		})
	}))
}
GO_EOF
} > main.go

echo "== go get 各模块的 $VERSION =="
for p in "$MOD" $PKGS; do
  echo "  $p@$VERSION"
  GOFLAGS=-mod=mod go get "$p@$VERSION" >/dev/null
done

echo "== 编译 =="
go mod tidy >/dev/null
go build -o verify . 
echo "  ✓ 装得上、编得过"

echo "== 起一遍：启动 → 请求 → 退出 =="
./verify --config=/dev/null >out.txt 2>&1 &
pid=$!
trap 'kill -9 "$pid" 2>/dev/null; rm -rf "$DIR"' EXIT

i=0
until curl -sf http://127.0.0.1:8080/ping >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -lt 50 ] || { echo "✗ 服务没起来"; cat out.txt; exit 1; }
  sleep 0.2
done
echo "  ✓ /ping 通了"

kill -TERM "$pid"
i=0
while kill -0 "$pid" 2>/dev/null; do
  i=$((i + 1))
  [ "$i" -lt 100 ] || { echo "✗ 收到 SIGTERM 之后没退出"; cat out.txt; exit 1; }
  sleep 0.2
done
echo "  ✓ 收到 SIGTERM 之后干净退出"

# 框架每跑一个钩子记一行 starting / stopping，字段 hook=包名.函数名。
# 从前这里 grep 的是 "closing"，命中的其实是收到信号那一行
# （"shutdown signal received, closing gracefully ..."），一个组件都没关也照样过。
#
# 日志前半截是 xlog 装好之前的 slog 默认格式（... INFO starting hook=xlog.initXLog），
# 后半截是 xlog 的 JSON（"msg":"stopping","hook":"xredis.closeXRedis"），两种都认。
# 取到包名为止：一个包可能登记多个启动钩子，配对的单位是包
hooks() {
  sed -n -e "s/.*\"msg\":\"$1\",\"hook\":\"\([^\"]*\)\".*/\1/p" \
         -e "s/.* INFO $1 hook=\([^ ]*\).*/\1/p" out.txt | cut -d. -f1 | uniq
}
hooks starting > started.txt
hooks stopping > stopped.txt
# 两边都有的包：停止顺序必须正好是启动顺序倒过来
awk '{ l[NR] = $0 } END { for (i = NR; i > 0; i--) print l[i] }' started.txt | grep -Fxf stopped.txt > want.txt || true
grep -Fxf started.txt stopped.txt > got.txt || true
[ "$(wc -l < got.txt)" -ge 2 ] || { echo "✗ 没看到组件关闭的日志"; cat out.txt; exit 1; }
cmp -s want.txt got.txt || { echo "✗ 关闭顺序不是启动顺序的逆序"; echo "  启动：$(tr '\n' ' ' < started.txt)"; echo "  关闭：$(tr '\n' ' ' < stopped.txt)"; exit 1; }
# 日志最先起、最后关，其余组件的关闭日志才写得出去
[ "$(tail -n 1 got.txt)" = "xlog" ] || { echo "✗ 最后关的不是 xlog：$(tr '\n' ' ' < got.txt)"; exit 1; }
echo "  ✓ 组件被逆序关闭：$(tr '\n' ' ' < got.txt)"
echo
echo "✓ $VERSION 验证通过"
}
if [ "$MODE" = "--verify" ]; then verify; exit 0; fi

run() {
  echo "  \$ $*"
  if [ "$MODE" = "--apply" ]; then
    "$@"
  fi
}

echo "== 1. 确认工作区干净 =="
[ -z "$(git status --porcelain)" ] || { echo "✗ 工作区有未提交的改动，先提交或暂存"; exit 1; }

# 输出不吞掉：红了的话要看的正是那几行
echo "== 2. 跑一遍检查、测试和 e2e =="
scripts/check.sh
scripts/test.sh -count=1
# 单元测试里没有数据库；连真实服务的那一层只有 e2e 看得到，发出去之前必须绿过一次
if [ -n "$E2E_PASSED" ]; then
  echo "  ⚠ 跳过 e2e：--e2e-passed，你担保这个提交的 e2e 刚跑绿过"
else
  scripts/e2e.sh
fi
echo "  ✓ 通过"

echo "== 3. 把各子模块开发用的 replace 换成真实版本号 =="
# replace 指向仓库内的相对路径，只在本地开发有意义。
# 消费者的构建会忽略依赖里的 replace，所以留着不会出错，
# 但那意味着子模块 require 的是 v0.0.0，谁都拉不到。
#
# 用 go mod edit 而不是 sed：require 块的缩进、子模块路径后缀这些细节
# 手写正则很容易弄错，而弄错的后果是发出去一个装不上的版本。
#
# inrepo Require|Replace go.mod：列出 require（路径@版本）/ replace（路径）里仓库内的模块。
# 两张表要分开取：从前从整份 -json 里 grep 路径，只出现在 replace 里的
# （xgin 替换了 xtrace 却不 require 它）也被补了一条 require，发出去的 go.mod 平白多一个依赖
inrepo() {
  GOWORK=off go mod edit -json "$2" | python3 -c '
import json, sys
mod, key = sys.argv[1], sys.argv[2]
for e in json.load(sys.stdin).get(key) or []:
    p = e["Path"] if key == "Require" else e["Old"]["Path"]
    if p == mod or p.startswith(mod + "/"):
        print(p + "@" + e["Version"] if key == "Require" else p)' "$MOD" "$1"
}
for m in $MODS; do
  echo "  $m/go.mod"
  # require 了的钉成 $VERSION：-require 直接改写版本号，原来是 v0.0.0 还是
  # go 工具自己补的伪版本（xgin 里 xmetric 的 v0.0.0-2026…）都一样
  for dep in $(inrepo Require "$m/go.mod"); do
    run env GOWORK=off go mod edit -require="${dep%@*}@$VERSION" "$m/go.mod"
  done
  for dep in $(inrepo Replace "$m/go.mod"); do
    run env GOWORK=off go mod edit -dropreplace="$dep" "$m/go.mod"
  done
  if [ "$MODE" = "--apply" ]; then
    # 改完再读一遍：仓库内的 require 只剩 $VERSION，replace 一条不剩
    left=$(inrepo Replace "$m/go.mod"; inrepo Require "$m/go.mod" | grep -v "@$VERSION\$" || true)
    [ -z "$left" ] || { echo "✗ $m/go.mod 里还有没钉好的仓库内依赖：$left"; exit 1; }
  fi
done

# 去掉 replace 之后还能不能编译，这里验不了：tag 还没推，依赖拉不到。
# 那一步交给推送之后的 --verify
echo "== 4. 提交并打 tag =="
run git add -A
run git commit -m "release: $VERSION"
run git tag "$VERSION"
for m in $MODS; do
  run git tag "$m/$VERSION"
done

echo "== 5. 把开发用的 replace 还原回来，单独一个提交 =="
# tag 要的是去掉 replace 的那一版，但分支不能停在那里：没有 replace，
# 子模块 require 的就是刚发出去的核心，scripts/ 下的检查和测试都是
# GOWORK=off 跑的，从此测的是已发布的代码，而不是工作区里正在改的这份——
# 改了核心、子模块测试照过，因为它根本没用到你的改动。
#
# 直接取发布提交之前那一版 go.mod，而不是再拼一遍 replace：那一版就是
# 发布前测试通过的状态，照原样拿回来不会有「拼错了一条」这种事
run git checkout "$VERSION^" -- $(printf '%s/go.mod ' $MODS)
run git commit -m "chore: restore the workspace replace directives after $VERSION"
if [ "$MODE" = "--apply" ]; then
  # 发布提交只该动了这些 go.mod：还原之后整棵树必须和发布之前一模一样
  git diff --quiet "$VERSION^" HEAD || { echo "✗ 还原之后和发布前不一致，检查 $VERSION 那个提交改了什么"; exit 1; }
  echo "  ✓ 工作区回到发布前的状态，tag 仍指向去掉 replace 的那个提交"
fi

echo
if [ "$MODE" = "--apply" ]; then
  cat <<TIP

已在本地打好 tag，并在它后面补了一个还原 replace 的提交。确认无误后推送：

  git push origin main --tags

两个提交一起推：tag 指向去掉 replace 的发布提交，main 停在还原之后的那个。

推送之后 tag 就被 module proxy 永久缓存了，删不掉，只能再发一版盖过去。
推完必须验证一遍别人装不装得上 —— CI 跑的是工作区里的代码，
证明不了发布出去的版本能装：

  scripts/release.sh $VERSION --verify
TIP
else
  echo "以上是 --apply 时会执行的命令，当前什么都没改。"
fi
