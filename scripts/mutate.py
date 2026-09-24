#!/usr/bin/env python3
# 变异测试：把每一条承诺对应的代码改坏，看有没有测试会失败。
#
#   ./mutate.py             改坏、编译、跑测试，几分钟，要干净工作区
#   ./mutate.py --dry-run   只在内存里套一遍模式，不写文件、不跑 go，一秒以内
#
# 活下来的变异 = 一条没有牙齿的承诺：代码写着、文档写着，但改坏了没人知道。
# 这个仓库前后被外部 review 挑出过二十多个问题，事后归类，绝大多数都是
# 「承诺有、测试没有」。与其每次等人来挑，不如让它自己说出来。
#
# 这不是 CI 的一部分（跑一轮要几分钟，而且要改工作区），是改完一批
# 安全或生命周期相关的代码之后手动跑一次的东西。--dry-run 不一样：它只查
# 「每条变异的模式是不是恰好匹配 N 处」，check.sh 每次都跑，重构挪走了
# 代码当场就知道，不用等到下一次想起来跑全量。
#
# 只有「我们自己有代码在守」的承诺才适合放进来。像「ctx 能给整次逻辑请求
# 封顶」那种由依赖库保证的性质，这里没有哪一行可以改坏，硬写一条变异
# 只会让「活下来 = 缺测试」这个信号失真——那种承诺靠测试守着就行，
# 它防的是升级依赖时的回归，不是防我们自己改错。
#
# 每条变异是表里的一行：mutate(名字, 文件, 模块, 测试过滤, 改法...)，
# 改法只写 swap() / cut()。读文件、写回、「模式还匹配得上吗」的断言都在这里，
# 不用每条自己记得写。
import os
import signal
import subprocess
import sys


class Stale(Exception):
    """变异模式没对上：这条承诺这一轮根本没被检查"""


def swap(old, new, count=1):
    """把 old 换成 new，并断言它恰好出现 count 次。

    断言是重点，不是顺手加的：重构挪走一段代码之后，模式会悄悄匹配不到，
    这条承诺从此再也没被检查过，而整轮照报绿——这个仓库里真发生过两次。
    匹配到的处数不对同样要拦：多匹配一处就是改坏了两个地方，
    测试红了也说明不了是哪一条承诺在起作用。
    """
    def edit(s):
        if (n := s.count(old)) != count:
            raise Stale(f"模式匹配到 {n} 处、期望 {count} 处，这条变异要跟着代码改：\n{old}")
        return s.replace(old, new)
    return edit


def cut(start, end_after):
    """删掉从 start 开始、到 end_after 那一段结束为止的整块代码"""
    def edit(s):
        if (n := s.count(start)) != 1:
            raise Stale(f"起点匹配到 {n} 处、期望 1 处：\n{start}")
        i = s.index(start)
        if (j := s.find(end_after, i)) < 0:
            raise Stale(f"找不到终点：\n{end_after}")
        return s[:i] + s[j + len(end_after):]
    return edit


PLAN = []


def section(title):
    PLAN.append(title)


def mutate(name, file, module, filt, *edits):
    PLAN.append((name, file, module, filt, edits))


def apply(name, file, edits):
    """在内存里套一遍改法，返回 (原文, 改后)；模式失效时打印原因、返回 None"""
    orig = open(file, encoding="utf-8").read()
    try:
        s = orig
        for e in edits:
            s = e(s)
    except Stale as err:
        print(f"  ? {name}（变异没应用上，改坏的位置可能已经不在了）\n      {err}")
        return None
    # 改不动和「改坏了没人发现」一样严重：这条承诺这一轮根本没被检查，
    # 而脚本从前照样报绿。重构挪走了一段代码，对应的变异就这样悄悄失效了
    if s == orig:
        print(f"  ? {name}（变异没改动任何东西，模式失效了）")
        return None
    return orig, s


def go_test(module, *args):
    return subprocess.run(["go", "test", "-count=1", *args, "./..."], cwd=module,
                          env=dict(os.environ, GOWORK="off"), capture_output=True, text=True)


def run(name, file, module, filt, edits, dry_run):
    """返回 killed / survived / stale / broken 之一"""
    got = apply(name, file, edits)
    if got is None:
        return "stale"
    if dry_run:
        return "killed"
    orig, s = got
    # 只写回自己动过的这一个文件，而且放在 finally 里：Ctrl-C 是异常、
    # SIGTERM 在 main 里换成了 SystemExit，两种打断都会走到这里。
    # 从前的 shell 版本靠 trap，被 kill 时 trap 不一定跑得到
    try:
        open(file, "w", encoding="utf-8").write(s)
        # 先只编译不跑（-exec true：测试二进制照常编出来、连同 go test 自带的那组
        # vet 检查，但交给 true 去「执行」）。编不过的变异从前被算作「被杀掉」：
        # 测试红了，可红的原因是 declared and not used，不是哪条测试察觉了行为变化——
        # 这条承诺这一轮同样根本没被检查，和 stale 一样严重，单独报出来
        build = go_test(module, "-exec", "true")
        if build.returncode != 0:
            print(f"  ! {name}（变异后编译不过，这一轮根本没检查到）")
            lines = (build.stdout + build.stderr).splitlines()
            for line in [l for l in lines if not l.startswith(("ok ", "FAIL ", "? "))][:3]:
                print(f"      {line}")
            return "broken"
        # -timeout 是必须的：有些变异会让测试挂住而不是失败（比如把 ctx 换成
        # Background，等信号的那一步就永远等不到）。没有上限的话一个这样的变异
        # 就能把整轮跑死在那里。挂住同样说明测试察觉到了，算作被杀掉
        if go_test(module, "-timeout", "90s", "-run", filt).returncode == 0:
            print(f"  ✗ {name} —— 改坏了但测试全过")
            return "survived"
        print(f"  ✓ {name}")
        return "killed"
    finally:
        open(file, "w", encoding="utf-8").write(orig)


def main():
    os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))  # 仓库根
    sys.stdout.reconfigure(line_buffering=True)  # 一轮几分钟，接到 tee / 文件时也要逐条看到进度
    dry_run = sys.argv[1:] == ["--dry-run"]
    if sys.argv[1:] and not dry_run:
        sys.exit("用法：scripts/mutate.py [--dry-run]")
    if not dry_run and subprocess.run(["git", "status", "--porcelain"],
                                      capture_output=True, text=True, check=True).stdout:
        sys.exit("✗ 工作区不干净，先提交或暂存")
    # SIGTERM 默认直接终止进程、finally 不跑；换成 SystemExit，被改坏的那个文件照样写回
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    got = {"killed": 0, "survived": 0, "stale": 0, "broken": 0}
    for p in PLAN:
        if isinstance(p, str):
            if not dry_run:
                print(f"== {p} ==")
        else:
            got[run(*p, dry_run)] += 1
    total = sum(got.values())
    if got["killed"] == total:
        if dry_run:
            print(f"✓ {total} 条变异的模式都恰好对得上")
        else:
            print(f"\n✓ {total} 条承诺全部有测试盯着")
        return
    print()
    if got["survived"]:
        print(f"✗ {got['survived']} 条改坏了也没人发现")
    if got["stale"]:
        print(f"✗ {got['stale']} 条的变异模式已经失效，这一轮根本没检查到（多半是重构挪走了那段代码）")
    if got["broken"]:
        print(f"✗ {got['broken']} 条变异后编译不过，测试红是因为编不过而不是察觉了行为变化，改变异让它编得过")
    print(f"  —— 共 {total} 条")
    sys.exit(1)


# ---------------------------------------------------------------------------
# 变异表。每条前面的注释写「这条承诺防的是哪次真出过的事」

section("配置")
mutate("日志按配置的时区渲染", "xlog/xlog.go", ".", "TestNew_日志|TestNew_时区",
       swap('ReplaceAttr: inLocation(loc)}', 'ReplaceAttr: nil}\n\t_ = loc'))
mutate("时区加载不到直接失败", "xlog/xlog.go", ".", "TestNew_时区",
       swap('''\tloc, err := time.LoadLocation(name)
\tif err != nil {''', '''\tloc, err := time.LoadLocation(name)
\tif false {'''))
# 文件名最细到分钟：0s 实际每分钟一个文件、30s 两个周期撞同一个名字，都是配了 A 跑的是 B
mutate("轮转周期短于一分钟启动失败", "xlog/config.go", ".", "TestNew_轮转周期",
       swap('if c.RotateTime < time.Minute {', 'if c.RotateTime < 0 {'))
mutate("MaxAge 为负启动失败", "xlog/config.go", ".", "TestNew_MaxAge", swap('if c.MaxAge < 0 {', 'if false {'))
# 校验函数本身是对的不算数，New 得真的调它
mutate("New 真的校验了文件输出的配置", "xlog/xlog.go", ".", "TestNew",
       swap('\tif err := cfg.File.validate(); err != nil {\n\t\treturn nil, nil, xerror.New("xlog", "config", err)\n\t}\n', ''))
mutate("Perm 认 0o644 写法", "xlog/xlog.go", ".", "TestParsePerm", swap('strings.TrimPrefix(s, "0o")', 's'))
# 关掉文件之后 slog.Default() 还指着它的话，Run 返回的错误、被丢下的停止钩子打的日志全没了
mutate("关掉日志文件之后改写 stderr", "xlog/xlog.go", ".", "TestCloseXLog",
       swap('\tslog.SetDefault(slog.New(slog.NewJSONHandler(os.Stderr, nil)))\n\treturn closer.Close()', '\treturn closer.Close()'))
# {Name} 上已经有普通文件时，Rename 会把它原子地换成符号链接，旧日志一声不响就没了
mutate("同名普通文件不被符号链接覆盖", "xlog/rotate.go", ".", "TestRotateWriter",
       cut('\tif occupied(w.linkName) {\n\t\treturn nil, xerror.Newf(', 'change File.Name", w.linkName)\n\t}\n'))
mutate("运行中链接位置被占了也不覆盖", "xlog/rotate.go", ".", "TestRotateWriter",
       cut('\tif occupied(w.linkName) {\n\t\twarnf(', 'not updating the link", w.linkName)\n\t\treturn\n\t}\n'))
# app.log.bak、app.log.1.gz 都匹配 base.*，按 mtime 判断就会被当成过期日志删掉
mutate("清理只删自己命名的文件", "xlog/rotate.go", ".", "TestRotateWriter",
       swap('\t\treturn !t.Add(period).After(cutoff)\n\t}\n\treturn false\n}', '\t\treturn !t.Add(period).After(cutoff)\n\t}\n\treturn true\n}'))
mutate("换过粒度之后旧文件照样清", "xlog/rotate.go", ".", "TestRotateWriter",
       swap('\t\tt, err := time.ParseInLocation(l.layout, suffix, w.clock().Location())',
     '\t\tif l.layout != w.layout {\n\t\t\tcontinue\n\t\t}\n\t\tt, err := time.ParseInLocation(l.layout, suffix, w.clock().Location())'))
# 旧粒度的文件按现在的 RotateTime 算周期的话，从按天改成按小时之后，
# 今天那个还在写的按天文件零点一过就被当成过期删掉
mutate("旧粒度的文件按它自己那一档的周期算过期", "xlog/rotate.go", ".", "TestRotateWriter",
       swap('\t\tperiod := l.period\n\t\tif l.layout == w.layout {\n\t\t\tperiod = w.rotate\n\t\t}\n', '\t\tperiod := w.rotate\n'))
# 只在轮转时清的话，按天轮转、一天重启几次的服务永远等不到那次轮转
mutate("打开时就清理一次过期文件", "xlog/rotate.go", ".", "TestRotateWriter",
       swap('\tw.purge(w.currentName)\n\treturn w, nil', '\treturn w, nil'))
mutate("片段也有 profile 变体", "internal/config/source.go", ".", "TestLoad",
       swap('nested, err := fileSet(target, d, false, profiles, seen, depth+1)',
     'nested, err := withImports(target, d, profiles, seen, depth+1)'))
mutate("重试的等待逐次翻倍", "xutil/convert.go", ".", "TestRetry", swap('\t\t\tbackoff = nextBackoff(backoff)\n', ''))
mutate("预算按退避上界算", "xutil/convert.go", ".", "TestRetryBudget",
       swap('''\tbudget := timeout * time.Duration(attempts)
\tbackoff := min(interval, maxBackoff) // 与 Retry 一致：第一次也封顶
\tfor i := 1; i < attempts; i++ {
\t\tbudget += backoff
\t\tbackoff = nextBackoff(backoff)
\t}
\treturn budget''', '\treturn timeout*time.Duration(attempts) + interval*time.Duration(attempts-1)'))
# 文档说每次最多等 maxBackoff，第一次等待从前原样用 interval
mutate("第一次等待也封顶", "xutil/convert.go", ".", "TestRetry_第一次等待也不超过上限",
       swap('\tvar last error\n\tbackoff := min(interval, maxBackoff)', '\tvar last error\n\tbackoff := interval'))
mutate("预算里的第一次退避也封顶", "xutil/convert.go", ".", "TestRetryBudget_第一次退避",
       swap('\tbackoff := min(interval, maxBackoff) // 与 Retry 一致', '\tbackoff := interval // 与 Retry 一致'))
mutate("重试的等待带抖动", "xutil/convert.go", ".", "TestJitter",
       swap('\treturn time.Duration(rand.Int64N(int64(d) + 1))', '\treturn d - time.Duration(rand.Int64N(2))'))
# 密码错、库不存在也照样重试满整轮，启动白白拖长几十秒
mutate("永久错误不再重试", "xutil/convert.go", ".", "TestRetry_永久错误|TestRetry_包在别的错误里",
       swap('errors.As(last, &p) {', 'errors.As(last, &p) && false {'))
mutate("永久错误返回去掉标记的原错误", "xutil/convert.go", ".", "TestRetry_永久错误|TestRetry_包在别的错误里",
       swap('\t\t\treturn p.err\n\t\t}\n', '\t\t\treturn last\n\t\t}\n'))
# 退避期间被取消时只报上一次的「连不上」，启动路径会把按要求退出当成故障
mutate("退避期间被取消时如实报告取消", "xutil/convert.go", ".", "TestRetry",
       swap('return fmt.Errorf("%w, last attempt failed: %w", err, last)', 'return fmt.Errorf("%v, last attempt failed: %w", err, last)'))
# yaml.v3 对没写 tag 的字段只认全小写：照着字段名写，报的是「field Endpoint not found」，
# 字段明明就叫这个。提示丢了的话，使用者只能对着一个自相矛盾的报错发愣
mutate("字段没写 tag 时报错说怎么改", "internal/config/strict.go", ".", "TestDecodeStrict",
       swap('\tif s.untagged[key] {', '\tif false {'))
# yaml 的类型错误带着值的前几个字符，${VAR} 又是凭证的推荐写法：密码填错了字段，
# 它的一截就进了启动日志。调用点和「记下展开了哪些值」各打一条
mutate("占位符展开出来的值不进报错", "internal/config/strict.go", ".", "TestUnmarshal_占位符展开出来的值不进报错",
       swap('e = c.at(n) + ": " + c.redact(n, msg)', 'e = c.at(n) + ": " + msg'))
mutate("加载时记下占位符展开出来的值", "internal/config/config.go", ".", "TestUnmarshal_占位符展开出来的值不进报错",
       swap('\texpand(root, &missing, values)', '\texpand(root, &missing, nil)'))
# 变量的值恰好是 null / ~ 时，重新判定把它当成「没写」，字段悄悄留在默认值上
mutate("展开出来的 null 写法仍是字符串", "internal/config/config.go", ".", "TestLoad_占位符的值是null写法",
       swap('\t\tif n.ShortTag() == "!!null" {\n\t\t\tn.Tag = "!!str"\n\t\t}\n', ''))
# 严格解码从前是把节点序列化成文本再解的：报的是那段文本的行号，也没有文件名。
# 现在每条报错都记在节点名下——拼错记在 key 上、类型错误记在值上，
# 「记下节点来自哪个文件」、合并出来的副本跟着第一个 key 走，各打一条
mutate("字段拼错时报的是那个 key 的行号", "internal/config/strict.go", ".", "TestLoad_字段写错",
       swap('c.errs = append(c.errs, c.at(k)+": "+s.notFound(k.Value, t))', 'c.errs = append(c.errs, c.at(n)+": "+s.notFound(k.Value, t))'))
mutate("类型错误也报出配置文件和那一行", "internal/config/strict.go", ".", "TestLoad_字段写错",
       swap('if msg, ok := strings.CutPrefix(e, fmt.Sprintf("line %d: ", n.Line)); ok {', 'if msg, ok := e, false; ok {'))
mutate("字段写错时报出是哪个文件", "internal/config/config.go", ".", "TestLoad_字段写错",
       swap('remember(f.node, f.path, from)', 'remember(f.node, "", from)'))
mutate("合并出来的块跟着第一个 key 认文件", "internal/config/strict.go", ".", "TestLoad_字段写错",
       swap('for ; n != nil; n = first(n) {', 'for ; n != nil; n = nil {'))
mutate("字段拼错要启动失败", "internal/config/strict.go", ".", "TestLoad",
       swap('\t\tif ft == nil {\n\t\t\tc.errs = append(c.errs, c.at(k)+": "+s.notFound(k.Value, t))\n',
            '\t\tif ft == nil {\n'))
# 未知字段是自己按类型查的，认字段的规则一处和 yaml.v3 不一样，要么拼错放行、要么合法的配置起不来
mutate("<< 并进来的字段照样认", "internal/config/strict.go", ".", "TestLoad_锚点",
       swap('if k.Kind == yaml.ScalarNode && k.Value == "<<" && k.ShortTag() == "!!merge" {', 'if false {'))
mutate(",inline 的结构体摊平来认", "internal/config/strict.go", ".", "TestDecodeStrict_字段规则",
       swap('\t\t\t\ts.add(ft) //', '\t\t\t\t_ = ft //'))
mutate(",inline 的 map 收下认不出的 key", "internal/config/strict.go", ".", "TestDecodeStrict_字段规则",
       swap('\t\t\t\ts.inline = ft.Elem()', '\t\t\t\t_ = ft'))
# 自己会解的元素要在检查那一遍里就试解：等到最后才解的话，外面有一处写错，
# 元素里的问题要改完、重启一次才看得见
mutate("集合元素里的问题和外面的一起报", "internal/config/strict.go", ".", "TestLoad_集合元素",
       swap('reflect.PointerTo(t).Implements(obsoleteType):\n\t\treturn c.try(n, t)',
            'reflect.PointerTo(t).Implements(obsoleteType):\n\t\treturn nil'))
mutate("占位符按替换后的内容判定类型", "internal/config/config.go", ".", "TestLoad",
       swap('\t\tif n.Value != before {\n\t\t\tretag(n)\n', '\t\tif n.Value != before {\n'))
# 合并按名字对齐 key，重复在那一步就被吞掉了，所以只能在每个文件解析时查。
# 打在调用点上：从前只有第一个文件的重复查得出来，profile 和 import 进来的全漏
mutate("每个文件里的重复 key 都要报错", "internal/config/source.go", ".", "TestLoad",
       cut('\tif err := checkDuplicates(&doc); err != nil {', 'return nil, xerror.Newf("xconfig", "config", "config %s: %w", path, err)\n\t}\n'))
# 只有注释的 profile 文件从前把前面所有文件整个替换掉，全部配置退回默认值
mutate("空文件叠上来不改变任何东西", "internal/config/merge.go", ".", "TestLoad",
       swap('return n == nil || n.Kind == 0 || isNull(n)', 'return n == nil || isNull(n)'))
mutate("叠加文件里的 null 不覆盖低优先级的值", "internal/config/merge.go", ".", "TestLoad",
       swap('return n == nil || n.Kind == 0 || isNull(n)', 'return n == nil || n.Kind == 0'))
mutate("占位符展开为空时保持默认值", "internal/config/config.go", ".", "TestLoad",
       swap('\tcase n.Value == "":\n\t\tn.Tag = "!!null"\n', '\tcase n.Value == "":\n'))
mutate("锚点可以跨顶层块引用", "internal/config/source.go", ".", "TestLoad",
       swap('\tbudget := maxResolvedNodes\n\tout, err := resolveAliases(&doc, &budget)', '\tout, err := &doc, error(nil)'))
mutate("Profiles.Active 收逗号分隔的字符串", "internal/config/config.go", ".", "TestLoad",
       swap('out = append(out, splitProfiles(a)...)', 'out = append(out, a)'))
mutate("Profiles 里的占位符会展开", "internal/config/config.go", ".", "TestLoad",
       swap('''\tvar missing []string
\texpand(node, &missing, nil)
\tif len(missing) > 0 {
\t\treturn nil, xerror.Newf("xconfig", "config",
\t\t\t"environment variables not set in %s of %s: %s", ProfilesKey, path, strings.Join(missing, ", "))
\t}
''', ''))
mutate("被引进来的文件里写了 Profiles 就报错", "internal/config/config.go", ".", "TestLoad",
       swap('if takeTopLevel(f.node, ProfilesKey) != nil {',
     'if p, _ := profilesOf(f.node, f.path); len(p) > 0 {'))
# config_schema.json 从前在编辑器里一个字段拼错都标不出来，单实例 / 多实例两种写法
# 却全被标红——下面四条各守一处它和运行时对不上的地方
# map 的 value 从零值开始解：不逐个铺默认值，多实例里没写的字段全成了零值
mutate("多实例写法里没写的字段保持默认", "internal/config/clients.go", ".", "TestUnmarshalClients",
       swap('\tc := defaults()\n', '\tvar c C\n'))
# node.Decode 不带严格检查，「字段拼错就启动失败」在多实例里悄悄失效
mutate("多实例写法里拼错也报错", "internal/config/clients.go", ".", "TestUnmarshalClients",
       swap('if err := DecodeStrict(n, &c); err != nil {', 'if err := n.Decode(&c); err != nil {'))
# 实例的报错点名是哪个实例，并且用 %w 保住 yaml 的类型错误
mutate("多实例写法里拼错时点名实例、保住类型错误", "internal/config/clients.go", ".", "TestUnmarshalClients_实例里拼错时报出",
       swap('fmt.Errorf("%s.%s: %w", clientsKey, name, err)', 'fmt.Errorf("%s.%s: %v", clientsKey, name, err)'))
mutate("多实例的每个实例都调一次 Validate", "internal/config/clients.go", ".", "TestUnmarshalClients_每个实例",
       swap('any(&c).(interface{ Validate() error })', 'any(c).(interface{ Validate() error })'))
# 调用点：三个集成读配置时要把自己的默认值交进去
mutate("xgorm 多实例铺的是自己的默认值", "xgorm/config.go", "./xgorm", "TestConfig_单实例写法",
       swap('xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)', 'xconfig.UnmarshalClients(ConfigKey, func() ClientConfig { return ClientConfig{} })'))
mutate("xredis 多实例铺的是自己的默认值", "xredis/config.go", "./xredis", "TestConfig_单实例写法",
       swap('xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)', 'xconfig.UnmarshalClients(ConfigKey, func() ClientConfig { return ClientConfig{} })'))
mutate("xcache 多实例铺的是自己的默认值", "xcache/config.go", "./xcache", "TestConfig_单实例写法",
       swap('xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)', 'xconfig.UnmarshalClients(ConfigKey, func() ClientConfig { return ClientConfig{} })'))
mutate("schema 里拼错的字段标红", "internal/schemagen/schema.go", "./internal/schemagen", "TestSchema|TestCheckDocs",
       swap('Properties: map[string]*node{}, AdditionalProperties: false}', 'Properties: map[string]*node{}}'))
mutate("schema 分得清单实例和多实例", "internal/schemagen/schema.go", "./internal/schemagen", "TestSchema|TestCheckDocs",
       swap('\t\t\tRequired:             []string{"Clients"},\n', ''))
mutate("schema 里 Import 写一个字符串也行", "internal/schemagen/schema.go", "./internal/schemagen", "TestSchema|TestCheckDocs",
       swap('stringOrList = []string{"string", "array"}', 'stringOrList = "array"'))
mutate("schema 里数字字段收占位符", "internal/schemagen/schema.go", "./internal/schemagen", "TestSchema|TestCheckDocs",
       swap('return &node{Type: []string{typ, "string"}, Pattern: placeholder}', 'return &node{Type: typ}'))
# 从前在整份文档里 grep 字段名，XLog.File.Name 没写也会因为 App.Name 写了而通过
mutate("文档按节检查字段", "internal/schemagen/docs.go", "./internal/schemagen", "TestCheckDocs",
       swap('\t\tif !regexp.MustCompile(`\\b` + regexp.QuoteMeta(f) + `\\b`).MatchString(text) {',
     '\t\tif !regexp.MustCompile(`\\b` + regexp.QuoteMeta(f) + `\\b`).MatchString(md) {'))
mutate("文档里的 YAML 示例要过得了 schema", "internal/schemagen/docs.go", "./internal/schemagen", "TestCheckDocs",
       swap('\t\t\tfor _, p := range problems(rs, doc) {', '\t\t\tfor _, p := range problems(rs, map[string]any{}) {'))

section("启动与退出")
# Stop 是可选的：实现了才调。打在调用点上——绕开类型断言，写了 Stop 的服务就停不下来
mutate("写了 Stop 的才调它", "xone.go", ".", "TestRun_逆序关闭|TestRun_只写Start",
       swap('\tif s, ok := r.(stopper); ok {\n', '\tif s, ok := any(nil).(stopper); ok {\n'))
# Stop 签名写错编译器不拦，那个 Stop 就永远不会被调到
mutate("Stop 签名写错时直接报错", "xone.go", ".", "TestRun_Stop签名写错",
       swap('if m, ok := t.MethodByName("Stop"); ok {', 'if m, ok := t.MethodByName("NoSuchMethod"); ok {'))
mutate("Stop 写在指针上却传了值时直接报错", "xone.go", ".", "TestRun_Stop写在指针上",
       swap('reflect.PointerTo(t).MethodByName("Stop")', 'reflect.PointerTo(t).MethodByName("NoSuchMethod")'))
mutate("列表整体替换不逐元素合并", "internal/config/merge.go", ".", "TestLoad",
       swap('''\tif base.Kind != yaml.MappingNode || override.Kind != yaml.MappingNode {
\t\treturn override
\t}''', '''\tif base.Kind == yaml.SequenceNode && override.Kind == yaml.SequenceNode {
\t\tout := *base
\t\tout.Content = append(append([]*yaml.Node{}, override.Content...), base.Content[len(override.Content):]...)
\t\treturn &out
\t}
\tif base.Kind != yaml.MappingNode || override.Kind != yaml.MappingNode {
\t\treturn override
\t}'''))
# 把这份文档挪到它 import 的那些后面，优先级就反过来了。
# 带上那行注释是为了锚定唯一一处：fileSet 里有一段一模一样的
# `out = append(out, nested...)` + return
mutate("import 进来的压过引它的", "internal/config/source.go", ".", "TestLoad",
       swap('\tout := []loaded{{path: path, node: doc}}\n', '\tvar out []loaded\n'),
       swap('''\t\t// import 进来的文件同样有 profile 变体，但变体不存在不算错
\t\tnested, err := fileSet(target, d, false, profiles, seen, depth+1)
\t\tif err != nil {
\t\t\treturn nil, err
\t\t}
\t\tout = append(out, nested...)
\t}
\treturn out, nil''', '''\t\tnested, err := fileSet(target, d, false, profiles, seen, depth+1)
\t\tif err != nil {
\t\t\treturn nil, err
\t\t}
\t\tout = append(out, nested...)
\t}
\tout = append(out, loaded{path: path, node: doc})
\treturn out, nil'''))
# 第三个参数是 variantRequired：主文件的 profile 变体必须存在
mutate("profile 文件不存在直接失败", "internal/config/source.go", ".", "TestLoad",
       swap('return fileSet(base, doc, true, Profiles(declared), seen, 0)', 'return fileSet(base, doc, false, Profiles(declared), seen, 0)'))
mutate("退出时生产者还在投递也不崩", "example/consumer/queue.go", "./example", "TestQueue",
       swap('''\tselect {
\tcase <-q.done: // 已经关了，丢掉这条
\tcase q.ch <- m:
\t}''', '\tq.ch <- m'),
       swap('\t\tclose(q.done)', '\t\tclose(q.ch)'))
mutate("第二个信号能终止卡住的进程", "xone.go", ".", "TestRun",
       swap('\t\t\tsignal.Stop(ch)\n\t\t\to.log().Info(','\t\t\to.log().Info(',1))
# Stop 要能真的做事。沿用被取消的 ctx 的话，每个关闭动作一进去就被拒绝——
# 在途请求没做完、注册中心那条记录没注销，等于没有优雅退出这回事
mutate("Stop 拿到的 ctx 不继承那次取消", "xone.go", ".", "TestRun_Stop拿到的ctx不继承那次取消",
       swap('context.WithTimeout(context.WithoutCancel(ctx), o.stopTimeout)','context.WithTimeout(ctx, o.stopTimeout)'))
# Stop 收了 ctx 却不看它：同步调的话 Run 永远返回不了，一个停止钩子都轮不到
# 到点之后不留那一截余量：看着截止时间返回的 Stop 报的错被丢掉，进程以 0 退出
mutate("看着截止时间返回的 Stop 报的错不丢", "xone.go", ".", "TestRun_看着截止时间返回的Stop",
       swap('case <-time.After(stopGrace):', 'default:'))
mutate("不看 ctx 的 Stop 挂不住退出", "xone.go", ".", "TestRun_Stop不看ctx",
       swap('errors.Join(first, stopServer(serverCtx, o, s))', 'errors.Join(first, safe("stop", func() error { return s.Stop(serverCtx) }))'))
# 服务只能用预算的前 2/3。让它用满整份的话，不肯退出的服务把时间吃光，
# 每个停止钩子一进去就判超时，Close() 被扔进没人等的协程，资源全留在原地
mutate("服务吃不掉留给停止钩子的那一段", "xone.go", ".", "TestShutdown|TestRun_服务迟迟不退出",
       swap('context.WithTimeout(stopCtx, o.stopTimeout-o.stopTimeout/3)', 'context.WithTimeout(stopCtx, o.stopTimeout)'))
# K8s 发 SIGTERM，Ctrl-C 发 SIGINT。漏掉任何一个，那条路径上就没有优雅退出
mutate("两种退出信号都监听", "xone.go", ".", "TestRun_两种退出信号都被监听",
       swap('[]os.Signal{syscall.SIGINT, syscall.SIGTERM}','[]os.Signal{syscall.SIGTERM}'))
# xlog 是在启动钩子里换掉全局 logger 的。Run 开头把它捕获一次的话，
# 之后所有框架日志都还写在旧的那个上——服务起来了，初始化日志一行看不到
mutate("框架日志每次重新取全局 logger", "xone.go", ".", "TestRun_框架日志跟着钩子换掉的全局logger走",
       swap('\tctx, stopSignals := notifyShutdown(o)','\to.logger = o.log()\n\tctx, stopSignals := notifyShutdown(o)'))
mutate("停止钩子受停止预算约束", "xone.go", ".", "TestShutdown",
       swap('err := runWithin(hookCtx, e)', 'err := safeHook(hookCtx, e)'))
# 一个卡住的钩子能用到整个截止时间的话，后面的钩子揣着过期的 ctx 进去当场判超时，
# 最后关的日志连关文件都来不及
mutate("钩子之间给排在后面的各留一份", "xone.go", ".", "TestShutdown",
       swap('end.Add(-time.Duration(len(todo)-1-i)*reserve)', 'end.Add(-time.Duration(len(todo)-1-i)*reserve*0)'))
mutate("启动期间收到信号就不启动服务", "xone.go", ".", "TestRun",
       swap('started, err := runStart(ctx, o)','started, err := runStart(context.Background(), o)'))
mutate("出错时问得出是谁报的", "xone.go", ".", "TestRun",
       swap('xerror.Newf("xone", "start", "%s: %w", e.Name, err)', 'xerror.Newf("xone", "start", "%s: %v", e.Name, err)'))
# Run 返回的是 errors.Join。只顺着单链往里走的话，Join 里排在后面的兄弟
# 从来没被看过：Is(Join(xgin 的错, xgorm 的错), "xgorm") 是 false
mutate("xerror.Is 走遍 Join 的每个分支", "xerror/xerror.go", ".", "TestIs",
       cut('\tcase interface{ Unwrap() []error }:\n', '\t\t\t\treturn true\n\t\t\t}\n\t\t}\n'))
# 一条错误只有一个框：从前每层都带完整的「xone {module} {op} failed, err=[...]」，
# 根因被埋在最里面的括号里。下面三条各守一处
mutate("同模块的里层只留 op 和原因", "xerror/xerror.go", ".", "TestNewf|TestNew_",
       swap('\tif !n.same {\n\t\treturn n.e.render(false)\n\t}', '\tif true {\n\t\treturn n.e.render(false)\n\t}'))
mutate("里层不再重复 xone 前缀", "xerror/xerror.go", ".", "TestNewf|TestNew_",
       swap('return n.e.render(false)', 'return n.e.render(true)'))
mutate("Newf 折叠参数里的 xerror", "xerror/xerror.go", ".", "TestNewf",
       swap('\t\t\town[i] = nest(module, xe)\n', '\t\t\town[i] = xe\n'))
# var e *Error 没判空就被包一层：报错路径上的 panic 会把真正的故障盖掉
mutate("New 收到带类型的 nil 不 panic", "xerror/xerror.go", ".", "TestNew_带类型的nil",
       swap('\t\tif xe == nil {\n\t\t\treturn &Error{Module: module, Op: op}\n\t\t}\n', ''))
mutate("Newf 参数里带类型的 nil 不 panic", "xerror/xerror.go", ".", "TestNewf_参数里带类型的nil",
       swap('if xe, ok := a.(*Error); ok && xe != nil {', 'if xe, ok := a.(*Error); ok {'))
mutate("Is 遇到带类型的 nil 不 panic", "xerror/xerror.go", ".", "TestNewf_参数里带类型的nil",
       swap('\t\tif xe == nil {\n\t\t\treturn false // 带类型的 nil', '\t\tif false {\n\t\t\treturn false // 带类型的 nil'))
mutate("同模块再包一层原样返回", "xerror/xerror.go", ".", "TestNew_",
       swap('\t\tif xe.Module == module {\n\t\t\treturn xe\n\t\t}\n', ''))
mutate("建连重试可以被取消", "xutil/convert.go", ".", "TestRetry",
       swap('\tif err := parent.Err(); err != nil {\n\t\treturn err\n\t}\n\n',''))
# panic 出来的是 error 时要用 %w 接住，否则 errors.Is 问不出根因。打在两个调用点上
mutate("建实例 panic 出来的 error 留在链上", "internal/xclient/xclient.go", ".", "TestBuild_new_panic",
       swap('"instance %q %w", name, panicked(r))', '"instance %q panicked: %v", name, r)'))
mutate("关实例 panic 出来的 error 留在链上", "internal/xclient/xclient.go", ".", "TestClose_panic",
       swap('"close %w", panicked(r))', '"close panicked: %v", r)'))
mutate("建实例 panic 不漏掉已建好的", "internal/xclient/xclient.go", "./xredis", "TestInitAll",
       swap('safeNew(ctx, r.module, name, cfgs[name], new)', 'new(ctx, cfgs[name])'))
mutate("一个实例建不起来就把已建好的全关掉", "internal/xclient/xclient.go", "./xredis", "TestInitAll",
       swap('\t\t\tcloseAll(r.module, closers)\n\t\t\treturn err\n','\t\t\treturn err\n'))
# 一个模块边界一个 xerror：再按 new 包一层，New 报的 config / connect
# 就被压进里层，errors.As 取出来的 op 永远是 new
mutate("建实例的错误不再套一层", "internal/xclient/xclient.go", "./xcache", "TestInstall",
       swap('\t\terr = named(module, name, err)', '\t\terr = xerror.Newf(module, "new", "instance %q: %w", name, err)'))
mutate("同模块的错误沿用原来的 op", "internal/xclient/xclient.go", ".", "TestBuild",
       swap('ok && xe.Module == module {', 'false && ok && xe.Module == module {'))

section("流程编排")
# 步骤只要写 Process 和 Rollback：Name 默认取类型名，Dependency 默认强依赖
mutate("没写 Name 时用类型名", "xflow/xflow.go", ".", "TestNew_",
       swap('name: typeName(p), dep: Strong}', 'name: "", dep: Strong}'))
mutate("没写 Dependency 时是强依赖", "xflow/xflow.go", ".", "TestNew_",
       swap('name: typeName(p), dep: Strong}', 'name: typeName(p), dep: Weak}'))
mutate("写了 Dependency 就用写的", "xflow/xflow.go", ".", "TestNew_", swap('\t\ts.dep = d.Dependency()\n', '\t\t_ = d\n'))
# 回滚的是已执行的那段前缀：失败的弱依赖算在内（它可能留下了副作用），
# 失败的强依赖不算（它没成）
mutate("失败的弱依赖也要回滚", "xflow/xflow.go", ".", "TestExecute", swap('\t\t\tn++ // 失败的弱依赖', '\t\t\t// 失败的弱依赖'))
mutate("失败的强依赖那一步不回滚", "xflow/xflow.go", ".", "TestExecute",
       swap('''\t\tf.rollback(ctx, data, f.steps[:n], res, m)
\t\treturn res
\t}
\treturn res''', '''\t\tf.rollback(ctx, data, f.steps[:min(n+1, len(f.steps))], res, m)
\t\treturn res
\t}
\treturn res'''))
mutate("被取消的流程不能报成功", "xflow/xflow.go", ".", "TestExecute",
       swap('\t\t\tif ctx.Err() == nil {\n\t\t\t\tcontinue\n\t\t\t}','\t\t\tcontinue'))
# 两处：notifyStep 和 notifyFlow 各有一个，都去掉才算关掉隔离
mutate("监控实现 panic 被隔离", "xflow/monitor.go", ".", "TestMonitor", swap('\tdefer recoverNotify()\n', '', count=2))
# 一个模块边界一个 xerror：safeProcess 自己再包一层，文本就套成两层 xflow
mutate("步骤 panic 只包一层 xflow", "xflow/xflow.go", ".", "TestExecute",
       swap('''err = &PanicError{Value: r, Stack: debug.Stack()}
\t\t}
\t}()
\treturn p.Process(ctx, data)''', '''err = xerror.New("xflow", "execute", &PanicError{Value: r, Stack: debug.Stack()})
\t\t}
\t}()
\treturn p.Process(ctx, data)'''))
mutate("步骤 panic 出来的 error 留在链上", "xflow/xflow.go", ".", "TestExecute",
       swap('\terr, _ := e.Value.(error)\n\treturn err', '\treturn nil'))
# 栈不进错误消息，就得有地方看得到：默认监控把它记成单独的字段
mutate("默认监控记下 panic 的调用栈", "xflow/monitor.go", ".", "TestMonitor",
       swap('attrs = append(attrs, "stack", string(pe.Stack))', '_ = pe'))
# 回滚沿用调用方的 ctx 的话，请求一超时补偿就全部失败——而补偿最需要执行的
# 恰恰是那时候。剥掉取消之后另给一份 RollbackTimeout，是 xflow 最核心的一条承诺
mutate("流程超时之后回滚仍有自己的预算", "xflow/xflow.go", ".", "TestRollback|TestExecute",
       swap('context.WithTimeout(context.WithoutCancel(ctx), cfg.RollbackTimeout)', 'context.WithTimeout(ctx, cfg.RollbackTimeout)'))
# 预算只在步骤之间查的话，一个不看 ctx 的 Rollback 能把 Execute 挂住：
# 50ms 的预算等了 2s，挂住的那一步还不在 RollbackErrors 里
mutate("不看 ctx 的补偿也受回滚预算约束", "xflow/xflow.go", ".", "TestRollback",
       swap('err := rollbackWithin(rbCtx, s.Processor, data)', 'err := safeRollback(rbCtx, s.Processor, data)'))
# 第一步之前就取消的流程从前也报 rolled back，让人去查一次不存在的补偿
mutate("一步都没回滚就不报 Rolled", "xflow/xflow.go", ".", "TestExecute",
       swap('\trbCtx, cancel := context.WithTimeout(', '\tres.Rolled = true\n\trbCtx, cancel := context.WithTimeout('))
# 直接解进 cfg：同一进程里上一次 Run 的值带进下一次，校验失败的值也留在了 cfg 上
mutate("配置每次从默认值解起", "xflow/config.go", ".", "TestLoadConfig",
       swap('\tc := DefaultConfig()\n\tif err := xconfig.Unmarshal(ConfigKey, &c)', '\tc := cfg\n\tif err := xconfig.Unmarshal(ConfigKey, &c)'))
mutate("校验失败的配置不生效", "xflow/config.go", ".", "TestLoadConfig",
       swap('\t\treturn err\n\t}\n\tcfg = c', '\t\tcfg = c\n\t\treturn err\n\t}\n\tcfg = c'))

section("HTTP 服务")
# 等多久只看调用方的 ctx（xone.Run 给的是服务那一段停止预算）。换掉它的话
# 挂住的请求让 Stop 一直不返回，「整个退出流程只有一份预算」就成了空话
mutate("服务不超过调用方给的截止时间", "xgin/xgin.go", "./xgin", "TestStop",
       swap('shutCtx, cancel := shutdownCtx(ctx)', 'shutCtx, cancel := shutdownCtx(context.WithoutCancel(ctx))'))
mutate("超时后强制断掉在途连接", "xgin/xgin.go", "./xgin", "TestStop",
       swap('\t\tif cerr := srv.Close(); cerr != nil {\n\t\t\tslog.Warn("xgin force close failed", "error", cerr)\n\t\t}\n',''))
# Close 只关连接、取消请求的 ctx，handler 的协程照跑。Close 完就返回的话，
# 正在收尾的 handler 还没返回，框架就去关数据库了
mutate("断连之后等 handler 真正返回", "xgin/xgin.go", "./xgin", "TestStop_断连之后|TestStop_handler不看ctx",
       swap('if n := g.waitHandlers(ctx); n > 0 {', 'if n := int64(0); n > 0 {'))
mutate("每个请求都记进在途计数", "xgin/xgin.go", "./xgin", "TestStop_断连之后",
       swap('Handler:           g.track(g.engine.Handler()),', 'Handler:           g.engine.Handler(),'))
# Shutdown 用满全部时间的话，Close 落下时预算已经花完，收尾的 handler 没人等
mutate("Shutdown 给等 handler 留出一截", "xgin/xgin.go", "./xgin", "TestStop_断连之后",
       swap('shutCtx, cancel := shutdownCtx(ctx)', 'shutCtx, cancel := context.WithCancel(ctx)'))
mutate("内置路由也走用户中间件", "xgin/xgin.go", "./xgin", "TestBuild",
       swap('\t\te.Use(middleware.Recover(g.recover))\n\t\te.Use(g.extra...)\n', '\t\te.Use(middleware.Recover(g.recover))\n'),
       swap('\t\tfor _, f := range g.routes {', '\t\te.Use(g.extra...)\n\t\tfor _, f := range g.routes {'))
mutate("默认不信任 X-Forwarded-For", "xgin/xgin.go", "./xgin", "TestBuild|TestLog",
       cut('\tif err := e.SetTrustedProxies(c.TrustedProxies); err != nil {',
    '_ = e.SetTrustedProxies([]string{})\n\t}\n'))

# h2c 原先靠 x/net 的 h2c.NewHandler：连接被劫持，Shutdown 约 100µs 就返回 nil、
# 在途请求照跑，框架紧接着去关数据库。换回那个写法，这条承诺就没了
mutate("h2c 的在途请求也等它做完", "xgin/xgin.go", "./xgin", "TestStop_h2c",
       swap('\t"github.com/gin-gonic/gin"\n', '\t"github.com/gin-gonic/gin"\n\t"golang.org/x/net/http2"\n\t"golang.org/x/net/http2/h2c"\n'),
       swap('\t\tHandler:           g.track(g.engine.Handler()),\n\t\tProtocols:         protocols(c),\n',
     '\t\tHandler:           h2c.NewHandler(g.track(g.engine.Handler()), &http2.Server{}),\n'))
mutate("不开 UseH2C 时不接受明文 HTTP/2", "xgin/xgin.go", "./xgin", "TestStart_不开h2c",
       swap('p.SetUnencryptedHTTP2(c.UseH2C && !c.tlsEnabled())', 'p.SetUnencryptedHTTP2(true)'))
# net/http 把 ReadHeaderTimeout 的 0 当成「退到 ReadTimeout」，而后者默认也是 0：
# 慢连接攻击的主要防线整个消失，配置文件看上去只是写了个 0
mutate("读请求头超时写 0 要拦住", "xgin/config.go", "./xgin", "TestValidate", swap('\t\tif d.val <= 0 {', '\t\tif d.val < 0 {'))
mutate("负的读写超时要拦住", "xgin/config.go", "./xgin", "TestValidate",
       swap('if c.ReadTimeout < 0 || c.WriteTimeout < 0 {', 'if false {'))
# gin 不拒绝不以 / 开头的路径，而是悄悄改写：留空就把指标挂到了根路径 / 上
mutate("指标路径不以 / 开头要启动失败", "xgin/config.go", "./xgin", "TestValidate|TestLoadConfig",
       swap('if c.Metric && !strings.HasPrefix(c.MetricPath, "/") {', 'if false {'))
# 配置文件里的 XGin 块由 StageServer 的钩子认领并校验（Unmarshal 调 Validate）。
# 丢掉这个错误的话，配错的值要拖到服务 Start 才报出来，那时别的组件都已经连好了
mutate("配错的 XGin 块在启动阶段就失败", "xgin/xgin.go", "./xgin", "TestLoadConfig",
       swap('\t_, err := fileConfig()\n\treturn err\n', '\t_, _ = fileConfig()\n\treturn nil\n'))
# CurrentConfig 和装配都靠它兜底：解到一半的非法配置里 TrustedProxies 可能正是 0.0.0.0/0
mutate("XGin 块不合法时退回默认值", "xgin/xgin.go", "./xgin", "TestCurrentConfig|TestEngine_配置文件不合法",
       swap('\tif err := xconfig.Unmarshal(ConfigKey, &c); err != nil {\n\t\treturn DefaultConfig(), err\n',
     '\tif err := xconfig.Unmarshal(ConfigKey, &c); err != nil {\n\t\treturn c, err\n'))
mutate("WithConfig 给的那份也要校验", "xgin/xgin.go", "./xgin", "TestStart_配置非法",
       swap('\tif err := g.override.Validate(); err != nil {', '\tif err := g.override.Validate(); false && err != nil {'))
# 监听地址、TLS 和 engine 上的中间件、信任的代理必须出自同一份配置。
# Start 自己再读一遍的话，装配之后换过的配置只生效一半
mutate("Start 用的是装配时的那份配置", "xgin/xgin.go", "./xgin", "TestStart_用的是装配时",
       swap('\tc := g.conf\n', '\tc, _ := g.cfg()\n'))
# gin.New 在 debug 模式下会打一段「切到 release」的警告。先建后设的话，
# 配成 release 的服务照样打出它（使用者的二进制里没设 GIN_MODE 时默认就是 debug）
mutate("Mode 在建 engine 之前设", "xgin/xgin.go", "./xgin", "TestBuild_Mode",
       swap('\t\tgin.SetMode(c.Mode)\n\t\te := gin.New()\n', '\t\te := gin.New()\n\t\tgin.SetMode(c.Mode)\n'))
# 元信息要推迟到第一次访问才填：标题默认取的 App.Name 要等 xapp 的启动钩子，
# Register 早于 xone.Run 时在那一刻就填，标题就不对
mutate("Swagger 元信息等 App 读好再填", "xginswagger/swagger.go", "./xginswagger", "TestRegister",
       swap('\tc, _ := fileConfig()\n\n', '\tc, _ := fileConfig()\n\tif info != nil {\n\t\tfill(info, c)\n\t}\n\n'),
       swap('once.Do(func() { fill(info, c) })', 'once.Do(func() {})'))
mutate("Swagger 挂在配置的前缀下", "xginswagger/swagger.go", "./xginswagger", "TestRegister",
       swap('e.GET(c.URLPrefix+route, serve)', 'e.GET(route, serve)'))
# 默认值是预填进结构体的：默认给了 Schemes，没写它也会盖掉注解里的 @schemes
mutate("Swagger 没写 Schemes 就沿用注解", "xginswagger/swagger.go", "./xginswagger", "TestConfig_没写Schemes",
       swap('\treturn Config{}\n', '\treturn Config{Schemes: []string{"https", "http"}}\n'))
# 校验只剩 Validate 这一处，靠 xconfig.Unmarshal 调到它。变异把 &c 换成一个没有方法的
# 同构类型：解码照旧，Validate 不再被调——这正是忘了导出、或者方法签名写错时的形状
mutate("Swagger 前缀格式不对要启动失败", "xginswagger/swagger.go", "./xginswagger", "TestLoadConfig",
       swap('xconfig.Unmarshal(ConfigKey, &c)', 'func() error { type raw Config; return xconfig.Unmarshal(ConfigKey, (*raw)(&c)) }()'))

section("中间件")
mutate("代理网段写错要启动失败", "xgin/config.go", "./xgin", "TestValidate", swap('if !isIPOrCIDR(p) {','if false {'))
mutate("指标的 method 标签收敛", "xgin/middleware/metric.go", "./xgin", "TestMetric",
       swap('normalizeMethod(c.Request.Method)', 'c.Request.Method'))
mutate("请求头里的凭证被遮掉", "xgin/middleware/redact.go", "./xgin", "TestRedact",
       swap('\t\tif set[name] {\n\t\t\tattrs = append(attrs, slog.String(k, Redacted))\n\t\t\tcontinue\n\t\t}\n','\t\t_ = set\n'))
mutate("关独立实例不影响全局链路", "xtrace/xtrace.go", "./xtrace", "TestClose",
       swap('\tif live == c {\n\t\tlive = nil\n\t}','\tlive = nil'))
# 初始化之后登记的处理器直接挂到在跑的 provider 上；漏了这一支，它就进了
# 再也没人读的待办队列，Span 照常产生、永远到不了上报端
mutate("初始化之后登记的处理器立即挂上", "xtrace/xtrace.go", "./xtrace", "TestAddSpanProcessor",
       swap('\t\tlive.tp.RegisterSpanProcessor(sp)\n\t\treturn\n', '\t\t_ = live.tp\n'))
mutate("配置在装配时落到 engine 上", "xgin/xgin.go", "./xgin", "TestBuild", swap('\t\tapplyConfig(e, c)\n', ''))
# 回调在配置落到 engine 上之后才跑，所以回调里明确设了的以回调为准。
# 两种改坏的写法：配置挪到回调之后落，或者 Start 时再落一遍（原先就是这样）
mutate("回调里的 engine 设置盖得过配置", "xgin/xgin.go", "./xgin", "TestBuild_回调",
       swap('\t\tapplyConfig(e, c)\n', ''),
       swap('\t\tfor _, f := range g.routes {\n\t\t\tf(e)\n\t\t}\n', '\t\tfor _, f := range g.routes {\n\t\t\tf(e)\n\t\t}\n\t\tapplyConfig(e, c)\n'))
mutate("Start 不再把配置落一遍", "xgin/xgin.go", "./xgin", "TestBuild_回调",
       swap('\tc := g.conf\n', '\tc := g.conf\n\tapplyConfig(g.engine, c)\n'))
# 开关写在配置里，读它们的是装配里的这几处。开关接不上是最难发现的一类 bug：
# 程序照常跑，只是那一项从来没生效
mutate("访问日志的开关读的是配置", "xgin/xgin.go", "./xgin", "TestLog_", swap('\t\tif c.Log {\n', '\t\tif true {\n', 2))
mutate("链路的开关读的是配置", "xgin/xgin.go", "./xgin", "TestTrace_", swap('\t\tif c.Trace {\n', '\t\tif true {\n'))
mutate("关掉指标就不挂指标中间件", "xgin/xgin.go", "./xgin", "TestMetric_",
       swap('\t\tif c.Metric {\n\t\t\te.Use(middleware.Metric())', '\t\tif true {\n\t\t\te.Use(middleware.Metric())'))
mutate("关掉指标就不注册端点", "xgin/xgin.go", "./xgin", "TestBuild_关掉指标",
       swap('\t\tif c.Metric {\n\t\t\te.GET(c.MetricPath, serveMetrics)', '\t\tif true {\n\t\t\te.GET(c.MetricPath, serveMetrics)'))
mutate("指标端点挂在配置的路径上", "xgin/xgin.go", "./xgin", "TestBuild_可以改指标路径",
       swap('e.GET(c.MetricPath, serveMetrics)', 'e.GET("/metrics", serveMetrics)'))
mutate("跳过日志的路径读的是配置", "xgin/xgin.go", "./xgin", "TestLogSkipPaths",
       swap('skip := append([]string{}, c.LogSkipPaths...)', 'skip := []string{}'))
# 接反了的话，只开了请求体的人，响应体（可能带着令牌）进了日志
mutate("请求体和响应体的开关各管各的", "xgin/xgin.go", "./xgin", "TestLogBody",
       swap('middleware.WithBody(c.LogRequestBody, c.LogResponseBody)', 'middleware.WithBody(c.LogResponseBody, c.LogRequestBody)'))
mutate("中文翻译的开关读的是配置", "xgin/xgin.go", "./xgin", "TestZHTranslations",
       swap('\t\tif c.ZHTranslations {', '\t\tif false {'))
# pgx 的错误原文里就是整串 DSN，它自己的打码只遮得住两种规整写法。
# 打在调用点上：绕开 parsePostgres 直接调 pgx，密码就跟着它的错误进了日志
mutate("PG 的 DSN 解不开时不把密码带进错误", "xgorm/dsn.go", "./xgorm", "TestResolvePostgres_",
       swap('\tpc, err := parsePostgres(dsn)\n', '\tpc, err := pgconn.ParseConfig(dsn)\n'))
# gorm 的 postgres 驱动建连用的是 pgx.ParseConfig，比 pgconn 多校验三项。
# 预检退回 pgconn 的话这三项写错会放行到建连，password = hunter2 跟着 pgx 的错误出去
mutate("PG 预检用的是建连时同一个解析器", "xgorm/dsn.go", "./xgorm", "TestNew_PG_pgx多校验",
       swap('\tcc, err := pgx.ParseConfig(dsn)\n\tif err == nil {\n\t\treturn &cc.Config, nil\n',
     '\t_ = pgx.ParseConfig\n\tcc, err := pgconn.ParseConfig(dsn)\n\tif err == nil {\n\t\treturn cc, nil\n'))
# gorm 用正则读原串、不解码：整个 query 重新编码，Asia/Shanghai 就成了 Asia%2FShanghai
mutate("URL 形式补参数不改写使用者的 query", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_URL形式补参数",
       swap('\tu.RawQuery = strings.Join(pairs, "&")\n',
     '\tu.RawQuery = strings.Join(pairs, "&")\n\tif all, err := url.ParseQuery(u.RawQuery); err == nil {\n\t\tu.RawQuery = all.Encode()\n\t}\n'))
mutate("补进 URL 的值不编码 /", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_URL形式补参数",
       swap('queryEscape(k)+"="+queryEscape(injects[k])', 'queryEscape(k)+"="+url.QueryEscape(injects[k])'))
mutate("PG 证书文件读不到时说清是哪个文件", "xgorm/dsn.go", "./xgorm", "TestResolvePostgres_",
       swap('\t\treturn nil, fmt.Errorf("cannot read a file named in the DSN: %w", pathErr)', '\t\treturn nil, errMalformedDSN'))
# 默认值垫在前面、不去判断 DSN 里写没写过，就是为了不再有这种检测。
# 变异把一个朴素的「写过就跳过」塞回来：密码里的 connect_timeout= 就能骗过它
mutate("密码里的参数名骗不过注入", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG密码里的参数名",
       swap('\t\tif skipTimeZone && gormTimeZone.MatchString(k+"=") {\n',
     '\t\tif strings.Contains(dsn, k+"=") || skipTimeZone && gormTimeZone.MatchString(k+"=") {\n'))
mutate("查询串不进访问日志", "xgin/middleware/log.go", "./xgin", "TestLog",
       swap('slog.String("path", c.Request.URL.Path),', 'slog.String("path", c.Request.URL.RequestURI()),'))
mutate("请求体只缓存前缀", "xgin/middleware/log.go", "./xgin", "TestSnapshotBody",
       swap('io.ReadAll(io.LimitReader(req.Body, maxRequestBody))','io.ReadAll(req.Body)',1))
mutate("预读时的错误接回下游", "xgin/middleware/log.go", "./xgin", "TestSnapshotBody",
       swap('\tif b.preErr != nil {\n\t\treturn 0, b.preErr\n\t}\n',''))

# ErrAbortHandler 是「断掉这个连接」的约定写法。兜住它的话本该中止的响应
# 被写成 500 发出去，还多一份毫无意义的 panic 栈
mutate("ErrAbortHandler 原样抛给 net/http", "xgin/middleware/middleware.go", "./xgin", "TestRecover",
       cut('\t\t\tif err == http.ErrAbortHandler {\n', '\t\t\t\tpanic(err)\n\t\t\t}\n'))
# 中止的请求往往已经写出了 200 的响应头：照读 c.Writer.Status() 的话，
# 被截断的响应在访问日志、指标、链路里都是一次成功
mutate("ErrAbortHandler 中止的请求登记下来", "xgin/middleware/middleware.go", "./xgin", "ErrAbortHandler中止",
       swap('\t\t\t\t_ = c.Error(http.ErrAbortHandler) //nolint:errcheck // 只是登记\n', ''))
mutate("访问日志把中止的请求记成 499", "xgin/middleware/log.go", "./xgin", "TestLog_ErrAbortHandler",
       swap('slog.Int("status", status(c)),', 'slog.Int("status", c.Writer.Status()),'))
mutate("指标把中止的请求记成 499", "xgin/middleware/metric.go", "./xgin", "TestMetric_ErrAbortHandler",
       swap('strconv.Itoa(status(c))', 'strconv.Itoa(c.Writer.Status())'))
mutate("链路把中止的请求记成错误", "xgin/middleware/trace.go", "./xgin", "TestTrace_ErrAbortHandler",
       swap('st := status(c)', 'st := c.Writer.Status()'))
# Content-Type 大小写不敏感：照字面比的话 Multipart/Form-Data 的文件内容整个进日志
mutate("上传和二进制流不读，不论大小写", "xgin/middleware/log.go", "./xgin", "TestSnapshotBody",
       swap('ct := strings.ToLower(req.Header.Get("Content-Type"))', 'ct := req.Header.Get("Content-Type")'))
# encoding/json 按 Unicode 折叠匹配字段名：{"ſecret":…} 绑得上 Secret。
# 只转小写的话字段名比对和预检都认不出它
mutate("字段名按 Unicode 折叠比对", "xgin/middleware/redact.go", "./xgin", "TestRedactBody_Unicode",
       swap('b.WriteRune(foldRune(r))', 'b.WriteRune(unicode.ToLower(r))'))
# 预检和字段名比对共用 sensitive，折叠本身由上一条盯着；这一条打在调用点上：
# 预检换成只转小写的朴素写法，{"ſecret":…} 就走快路径原样进日志
mutate("敏感词预检按 Unicode 折叠", "xgin/middleware/redact.go", "./xgin", "TestRedactBody_Unicode",
       swap('sensitive(s, words())', 'func(ws []string) bool { return slices.ContainsFunc(ws, func(w string) bool { return strings.Contains(strings.ToLower(s), w) }) }(words())'),
       swap('|| sensitive(body, ws)', '|| slices.ContainsFunc(ws, func(w string) bool { return strings.Contains(strings.ToLower(body), w) })'))
# path 特意不带查询串，Referer 却带着上一个页面的完整 URL
mutate("URL 类请求头去掉查询串", "xgin/middleware/redact.go", "./xgin", "TestRedactHeaders_URL",
       swap('\t\t\tv = stripQuery(v)\n', ''))
mutate("链路的 method 收敛", "xgin/middleware/trace.go", "./xgin", "TestTrace",
       swap('method := normalizeMethod(c.Request.Method)', 'method := c.Request.Method'))
# 原先是精确匹配：new_password、client_secret、sessionToken 原样进日志
mutate("JSON 字段名里带敏感词也遮", "xgin/middleware/redact.go", "./xgin", "TestRedactBody",
       swap('\t\t\tif sensitive(k, ws) {\n\t\t\t\tt[k] = Redacted', '\t\t\tif slices.Contains(ws, normalize(k)) {\n\t\t\t\tt[k] = Redacted'))
mutate("表单字段名里带敏感词也遮", "xgin/middleware/redact.go", "./xgin", "TestRedactBody",
       swap('\t\tif sensitive(k, ws) {\n\t\t\tvalues[k]', '\t\tif slices.Contains(ws, normalize(k)) {\n\t\t\tvalues[k]'))
# 名单永远列不全（Proxy-Authorization 就曾漏在外面），词表是兜底的那一层
mutate("请求头名字里带敏感词也遮", "xgin/middleware/redact.go", "./xgin", "TestRedactHeaders",
       swap('\t\tif sensitive(k, ws) {\n\t\t\tattrs = append(attrs, slog.String(k, Redacted))\n\t\t\tcontinue\n\t\t}\n', '\t\t_ = ws\n'))
# 预检认不出 api-key 的话，这种 body 走快路径原样进日志，根本到不了逐字段脱敏
mutate("敏感词预检忽略分隔符", "xgin/middleware/redact.go", "./xgin", "TestRedactBody",
       swap('sensitive(s, words())', 'func(ws []string) bool { return slices.ContainsFunc(ws, func(w string) bool { return strings.Contains(strings.Map(foldRune, s), w) }) }(words())'),
       swap('|| sensitive(body, ws)', '|| slices.ContainsFunc(ws, func(w string) bool { return strings.Contains(strings.Map(foldRune, body), w) })'))
mutate("脱敏后大整数不丢精度", "xgin/middleware/redact.go", "./xgin", "TestRedactBody", swap('\tdec.UseNumber()\n', ''))
mutate("脱敏后不转义 HTML 字符", "xgin/middleware/redact.go", "./xgin", "TestRedactBody",
       swap('\tenc.SetEscapeHTML(false)\n', ''))
mutate("JSON 后面跟着别的东西时整个遮掉", "xgin/middleware/redact.go", "./xgin", "TestRedactBody",
       swap('dec.Decode(new(any)) != io.EOF', '(dec.Decode(new(any)) != io.EOF && false)'))
# 全采样原先是裸的 AlwaysSample：上游 sampled=00，我们照样采、再以 -01 往下传
mutate("全采样也听上游的采样决定", "xtrace/xtrace.go", "./xtrace", "TestNew_全采样|TestSamplerOf",
       swap('\treturn sdktrace.ParentBased(sdktrace.TraceIDRatioBased(ratio))',
     '\tif ratio >= 1 {\n\t\treturn sdktrace.AlwaysSample()\n\t}\n\treturn sdktrace.ParentBased(sdktrace.TraceIDRatioBased(ratio))'))
mutate("采样率越界要拦住", "xtrace/config.go", "./xtrace", "TestNew_采样率越界",
       swap('if math.IsNaN(c.SampleRatio) || c.SampleRatio < 0 || c.SampleRatio > 1 {', 'if math.IsNaN(c.SampleRatio) && false {'))
# *trusted.com 原先会匹配 eviltrusted.com，内部令牌发给了谁都能注册的域名
mutate("通配域名只认星点开头", "xtrace/propagator.go", "./xtrace", "TestNewHeaderPropagator",
       swap('\t\tif err := checkDomain(d); err != nil {\n\t\t\treturn nil, err\n\t\t}\n', ''))
mutate("通配域名按点分隔的父域匹配", "xtrace/propagator.go", "./xtrace", "TestHeaderPropagator_域名规则",
       swap('strings.HasSuffix(h, "."+parent)', 'strings.HasSuffix(h, parent)'))
# OTEL_RESOURCE_ATTRIBUTES 写错一项、容器里查不到随机 UID，原先都让服务起不来
mutate("资源属性采集出错照常启动", "xtrace/xtrace.go", "./xtrace", "TestNew_资源",
       swap('\t\tslog.WarnContext(ctx, "xtrace some resource attributes', '\t\tpanic(err)\n\t\tslog.WarnContext(ctx, "xtrace some resource attributes'))
mutate("资源属性采集出错时用采到的那部分", "xtrace/xtrace.go", "./xtrace", "TestNew_资源",
       swap('\t\tslog.WarnContext(ctx, "xtrace some resource attributes', '\t\tres = resource.Empty()\n\t\tslog.WarnContext(ctx, "xtrace some resource attributes'))
# App.Name 没配时原先照样写进 service.name=""，把 OTel 的兜底名和 OTEL_SERVICE_NAME 一起盖掉
mutate("没配应用名时不写空的 service.name", "xtrace/xtrace.go", "./xtrace", "TestNew_资源",
       swap('if name := xapp.Name(); name != "" {', 'if name := xapp.Name(); true {'))
mutate("没配应用名时落到 OTel 的兜底名", "xtrace/xtrace.go", "./xtrace", "TestNew_资源",
       swap('\t\tresource.WithService(),\n', ''))
# resource.New 里后面的选项压过前面的：配置排在环境变量后面，部署方就改不动服务名
mutate("OTel 环境变量压过 App 配置", "xtrace/xtrace.go", "./xtrace", "TestNew_资源",
       swap('\t\tresource.WithAttributes(appAttributes()...),\n\t\tresource.WithFromEnv(),\n',
     '\t\tresource.WithFromEnv(),\n\t\tresource.WithAttributes(appAttributes()...),\n'))
# 链路关着时原先交回空操作的 Closer，登记的处理器（连同 exporter 的连接和协程）没人关
mutate("链路关着时照样关掉登记的处理器", "xtrace/xtrace.go", "./xtrace", "TestNew_关闭链路时关闭照样|TestCloseXTrace_关闭链路",
       swap('idle := sdktrace.NewTracerProvider(processors(procs)...)', 'idle := sdktrace.NewTracerProvider()'))
# 框架只剩 100ms 时照样等满自己的 ShutdownTimeout，后面的组件被挤掉
mutate("链路关闭听框架给的截止时间", "xtrace/xtrace.go", "./xtrace", "TestCloseXTrace",
       swap('return c.shutdown(ctx)', 'return c.Close()'))
mutate("链路关闭在调用方的 ctx 上收紧", "xtrace/xtrace.go", "./xtrace", "TestCloseXTrace",
       swap('context.WithTimeout(parent, c.timeout)', 'context.WithTimeout(context.Background(), c.timeout)'))
# 照单全收的话，公网客户端发一个 X-Tenant-Id 就被带进内网的每一次调用
mutate("透传 Header 只收可信对端发来的", "xtrace/propagator.go", "./xtrace", "TestHeaderPropagator_只收|TestNew_装好的",
       swap('\tif !fromTrustedPeer(carrier) {\n\t\tp.warnUntrusted(carrier)', '\tif false && !fromTrustedPeer(carrier) {\n\t\tp.warnUntrusted(carrier)'))
# baggage 和透传头是同一种东西，OTel 自带的那个谁发来的都收
mutate("baggage 只收可信对端发来的", "xtrace/propagator.go", "./xtrace", "TestNew_装好的Propagator只收可信对端的baggage",
       swap('\tif !fromTrustedPeer(carrier) {\n\t\t// 只告警一次', '\tif false {\n\t\t// 只告警一次'))
mutate("装的是只收可信对端的 baggage", "xtrace/xtrace.go", "./xtrace", "TestNew_装好的Propagator只收可信对端的baggage",
       swap('&trustedBaggage{}', 'propagation.Baggage{}'))
mutate("不可信对端带来 baggage 时只告警一次", "xtrace/propagator.go", "./xtrace", "TestTrustedBaggage",
       swap('b.warned.CompareAndSwap(false, true)', 'true'))
# 透传规则写错原先要等装 Propagator 才报；New 照样会报，所以要看错误出自读配置那一步
mutate("透传规则在读配置时就校验", "xtrace/config.go", "./xtrace", "TestInitXTrace_透传规则",
       swap('\tif c.forwardEnabled() {\n\t\tif _, err := newHeaderPropagator(', '\tif false && c.forwardEnabled() {\n\t\tif _, err := newHeaderPropagator('))
mutate("XTrace 块在读配置时就校验", "xtrace/xtrace.go", "./xtrace", "TestInitXTrace_透传规则",
       swap('xconfig.Unmarshal(ConfigKey, &c)', 'func() error { type raw Config; return xconfig.Unmarshal(ConfigKey, (*raw)(&c)) }()'))
mutate("不可信对端带来透传头时只告警一次", "xtrace/propagator.go", "./xtrace", "TestHeaderPropagator_不可信",
       swap('\tif p.warned.Load() {\n\t\treturn\n\t}\n', ''),
       swap('if p.warned.CompareAndSwap(false, true) {', 'if true {'))
# 「谁是自己人」只看 TrustedProxies。记号打错一次，要么伪造的头被带进内网，要么透传整个失效
mutate("对端在 TrustedProxies 里才算可信", "xgin/xgin.go", "./xgin", "TestBuild_只有|TestBuild_不配Trusted",
       swap('if trustedAddr(g.trusted, c.RemoteIP()) {', 'if len(g.trusted) > 0 {'))
mutate("可信网段来自装配时的配置", "xgin/xgin.go", "./xgin", "TestBuild_只有",
       swap('g.trusted = prefixes(c.TrustedProxies)', 'g.trusted = prefixes(nil)'))
mutate("装配时先判对端再开链路", "xgin/xgin.go", "./xgin", "TestBuild_只有",
       swap('e.Use(g.markTrustedPeer, middleware.Trace())', 'e.Use(middleware.Trace())'))
# XGin.Trace 只管 Span。原先关掉它连提取一起摘了，上游的链路标识和透传头都不收
mutate("XGin.Trace 关掉照样接上游的链路和透传", "xgin/xgin.go", "./xgin", "TestBuild_关掉Trace",
       swap('e.Use(g.markTrustedPeer, middleware.Propagate())', 'e.Use(g.markTrustedPeer)'))
mutate("XGin.Trace 关掉时可信规则不变", "xgin/xgin.go", "./xgin", "TestBuild_关掉Trace",
       swap('e.Use(g.markTrustedPeer, middleware.Propagate())', 'e.Use(middleware.Propagate())'))
mutate("Propagate 也把可信记号交给 xtrace", "xgin/middleware/trace.go", "./xgin", "TestTrace_对端可信",
       swap('WithContext(otel.GetTextMapPropagator().Extract(c.Request.Context(), inbound(c)))', 'WithContext(otel.GetTextMapPropagator().Extract(c.Request.Context(), propagation.HeaderCarrier(c.Request.Header)))'))
mutate("链路中间件把可信记号交给 xtrace", "xgin/middleware/trace.go", "./xgin", "TestTrace_对端可信",
       swap('\tif c.GetBool(peer.TrustedKey) {\n\t\treturn trustedCarrier{h}\n\t}\n', '\t_ = peer.TrustedKey\n'))
# 两个模块靠 TrustedPeer() 这个方法名接头，各自的单元测试只看得见自己那一半。
# 只改一边的名字，另一边的测试照样全绿——只有 example 里那条端到端的看得见
mutate("xgin 与 xtrace 的可信记号对得上", "xtrace/propagator.go", "./example", "TestXGin与XTrace",
       swap('\tt, ok := c.(interface{ TrustedPeer() bool })\n\treturn ok && t.TrustedPeer()', '\tt, ok := c.(interface{ TrustedUpstream() bool })\n\treturn ok && t.TrustedUpstream()'))

section("登记板")
# 档位是使用者理解生命周期的全部依据：日志最先起、链路早于客户端、
# 服务最后起。排错一档，表现是「实例比用它的东西晚就绪」，别处都测不出来
# 直接解进全局：前一次 Run 的服务名带进下一次，解码失败时写了一半的值也落了上去
mutate("xapp 解进新的默认值再换上", "xapp/xapp.go", ".", "TestLoadConfig_",
       swap('\tc := DefaultConfig()\n\tif err := xconfig.Unmarshal(ConfigKey, &c); err != nil {\n\t\treturn err\n\t}\n\tcfg = c\n\treturn nil',
            '\treturn xconfig.Unmarshal(ConfigKey, &cfg)'))
mutate("启动按档位升序", "internal/hook/hook.go", ".", "TestStartOrder|TestStopOrder|TestAddStart",
       swap('sort.SliceStable(out, func(i, j int) bool { return out[i].Stage < out[j].Stage })',
     'sort.SliceStable(out, func(i, j int) bool { return out[i].Name < out[j].Name })'))
# 同档内谁先登记谁先起。换成不稳定排序之后顺序会随实现变化，
# 而使用者是照着 import 的先后去理解它的
mutate("同档内保持登记顺序", "internal/hook/hook.go", ".", "TestStartOrder", swap('sort.SliceStable(out,', 'sort.Slice(out,'))
# 「声明一个档位就同时做到先启动、后关闭」这条承诺，全靠停止顺序是整体逆序
mutate("停止顺序是启动顺序的整体镜像", "internal/hook/hook.go", ".", "TestStopOrder",
       swap('''\tout := startOrder(in)
\tfor i, j := 0, len(out)-1; i < j; i, j = i+1, j-1 {
\t\tout[i], out[j] = out[j], out[i]
\t}
\treturn out''', '''\tout := append([]Entry(nil), in...)
\tsort.SliceStable(out, func(i, j int) bool { return out[i].Stage > out[j].Stage })
\treturn out'''))
# 配对键取末段包名的话，两个末段同名的包会被当成同一个：使用者自己包一层
# 叫 xlog 的包很常见，撞上之后它的启动钩子一失败，框架 xlog 的停止钩子
# 就跟着被跳过，日志写入器再也不 flush
mutate("配对键取完整 import path", "xhook/xhook.go", ".", "TestPkgOf|TestBeforeStart",
       swap('\t\treturn full[:slash+1+i]', '\t\treturn full[slash+1 : slash+1+i]'))
# 默认档落回 StageClient 的话，业务钩子和 xgorm 同档，谁先跑看包的初始化顺序：
# 业务包路径排在前面时，钩子里的 xgorm.C() 当场 panic
mutate("不写档位就落在业务那一档", "xhook/xhook.go", ".", "TestBeforeStart|TestAt",
       swap('o := options{stage: StageBusiness}', 'o := options{stage: StageClient}'))
# 客户端类集成要显式声明 StageClient：漏写就掉进默认的业务档，和用它的业务钩子同档
mutate("xgorm 在业务档之前就绪", "xgorm/xgorm.go", "./xgorm", "TestRegister_登记内容与框架对得上",
       swap('xhook.BeforeStart(initXGorm, xhook.At(xhook.StageClient))', 'xhook.BeforeStart(initXGorm)'))
# 停止钩子只在和它配对的启动钩子成功之后执行：不然启动到一半失败时，后面那些
# 资源根本没建起来，调它们的停止钩子只会在一堆空值上出错
mutate("启动失败的那一对不执行停止钩子", "xone.go", ".", "TestRun_组件初始化失败时回滚已初始化的部分",
       swap('e.Pair != 0 && !started[e.Pair] {', 'e.Pair != 0 && !started[e.Pair] && false {'))
# 之前没有启动钩子的停止钩子总会执行。Runnable 被拦下、配置读不出来时从前直接返回，
# 日志 flush 这类钩子一次都没跑。两条提前返回的路各打一条
mutate("Runnable 写错时不依赖启动的停止钩子照样执行", "xone.go", ".", "TestRun_Runnable写错时",
       swap('\tif err := checkRunnable(r); err != nil {\n\t\treturn errors.Join(err, stopWithin(o, nil))',
            '\tif err := checkRunnable(r); err != nil {\n\t\treturn err'))
mutate("配置读不出来时不依赖启动的停止钩子照样执行", "xone.go", ".", "TestRun_配置读不出来时",
       swap('\tif err := config.Ensure(o.configPath, o.log()); err != nil {\n\t\treturn errors.Join(err, stopWithin(o, nil))',
            '\tif err := config.Ensure(o.configPath, o.log()); err != nil {\n\t\treturn err'))
# 一起登记的就是一对：配成同包里最早的那个启动钩子的话，第二对建不起来时
# 它的停止钩子照样被调到，得处理「还没建起来」
mutate("停止钩子配的是之前最近登记的那个启动钩子", "internal/hook/hook.go", ".", "TestRun_停止钩子只认|TestRun_两个启动钩子|TestAddStop",
       swap('for i := len(start) - 1; i >= 0; i-- {', 'for i := 0; i < len(start); i++ {'))
# 配对只认同一个包：认错包的话，别的集成起不来时它的停止钩子被跳过，
# 或者它自己没建起来时停止钩子照样被调到
# Pair 为 0 表示「没有配对、总会执行」。序号从 0 编起的话，第一个登记的启动钩子
# 失败时，它的停止钩子会被当成不依赖启动的那种照样调到
# 只在启动钩子上写 At 的话，停止钩子从前掉回 StageBusiness：客户端先于业务被关掉
mutate("停止钩子不写档位时跟着配对的启动钩子", "xhook/xhook.go", "./xhook", "TestBeforeStop_不写档位",
       swap('Run: hook.Func(f), Inherit: !o.set}', 'Run: hook.Func(f), Inherit: false}'))
mutate("停止钩子显式写的档位以它为准", "xhook/xhook.go", "./xhook", "TestAt_指定档位覆盖默认值",
       swap('Run: hook.Func(f), Inherit: !o.set}', 'Run: hook.Func(f), Inherit: true}'))
mutate("配上对时才继承档位", "internal/hook/hook.go", ".", "TestAddStop_没显式指定档位",
       swap('\t\t\tif e.Inherit {\n\t\t\t\te.Stage = start[i].Stage\n\t\t\t}\n', ''))
# 测试辅助和 Run 是同一条规矩：启动失败的那一对不跑停止钩子，起来了的照样关
mutate("xonetest 只关配对的启动钩子成功了的", "xonetest/xonetest.go", ".", "TestStartHooks",
       swap('\t\t\tif e.Pair != 0 && !started[e.Pair] {', '\t\t\tif false {'))
mutate("xonetest 记下成功了的启动钩子", "xonetest/xonetest.go", ".", "TestStartHooks",
       swap('\t\tstarted[e.Seq] = true\n', ''))
mutate("xonetest 结束时清掉配置", "xonetest/xonetest.go", ".", "TestUseConfig",
       swap('\tt.Cleanup(config.Reset)\n', ''))
mutate("启动钩子的序号从 1 编起", "internal/hook/hook.go", ".", "TestAddStop",
       swap('e.Seq = len(start) + 1', 'e.Seq = len(start)'))
mutate("停止钩子只配同一个包的启动钩子", "internal/hook/hook.go", ".", "TestAddStop|TestRun_停止钩子只认",
       swap('\t\tif start[i].Pkg == e.Pkg {', '\t\tif true {'))
# 按钩子函数的名字认包的话，经辅助包登记的钩子全算在辅助包头上，彼此配成一团
mutate("登记它的包按调用栈上的 init 认", "xhook/xhook.go", ".", "TestBeforeStart",
       swap('Pkg: registrant(full)', 'Pkg: pkgOf(full)'))
# panic 出来的是 error 时要用 %w 接住，否则 errors.Is 问不出根因
mutate("钩子 panic 出来的 error 留在链上", "xone.go", ".", "TestRun_钩子panic",
       swap('return fmt.Errorf("panicked: %w", err)', 'return fmt.Errorf("panicked: %v", err)'))

section("客户端")
# 取不到实例时把原因说准：调早了、调晚了、没配、名字写错，要查的地方各不相同。
# 调早了从前和「没配」是同一句话，于是在 Run 之前取实例的人去翻配置文件
mutate("调早了不说成没配", "internal/xclient/xclient.go", ".", "TestGet_", swap('\tcase p == notStarted:\n', '\tcase false:\n'))
mutate("调晚了不说成没配", "internal/xclient/xclient.go", ".", "TestGet_", swap('\tcase p == closed:\n', '\tcase false:\n'))
# 打在调用点上：没配时不走 Build 的话，注册表一直停在「还没启动」，
# C() 就会把「没配」说成「调早了」
mutate("xgorm 没配也让注册表知道启动过了", "xgorm/xgorm.go", "./xgorm", "TestInitXGorm_没配时C说",
       swap('\t\treturn xclient.Build(ctx, reg, nil, build)\n', '\t\treturn nil\n'))
mutate("xredis 没配也让注册表知道启动过了", "xredis/xredis.go", "./xredis", "TestInitXRedis_没配时C说",
       swap('\t\treturn xclient.Build(ctx, reg, nil, build)\n', '\t\treturn nil\n'))
mutate("xcache 没配也让注册表知道启动过了", "xcache/xcache.go", "./xcache", "TestInitXCache_没配时C说",
       swap('\t\treturn xclient.Build(ctx, reg, nil, build)\n', '\t\treturn nil\n'))
# 乱序的桶能通过启动，然后在第一次 Observe 时 panic——在业务请求里；
# 不合规的 Namespace 不报错，导出时被悄悄转义，看板按原名查不到
mutate("指标配置说不通时启动就失败", "xmetric/xmetric.go", "./xmetric", "TestNew|TestInitXMetric",
       swap('\tif err := cfg.Validate(); err != nil {', '\tif err := cfg.Validate(); false && err != nil {'))
# New 照样校验，所以只看「启动失败」测不出来：要看错误出自读配置那一步
mutate("XMetric 块在读配置时就校验", "xmetric/xmetric.go", "./xmetric", "TestInitXMetric",
       swap('xconfig.Unmarshal(ConfigKey, &c)', 'func() error { type raw Config; return xconfig.Unmarshal(ConfigKey, (*raw)(&c)) }()'))
mutate("直方图的桶必须严格递增", "xmetric/config.go", "./xmetric", "TestNew",
       swap('if i > 0 && v <= b[i-1] {', 'if false && i > 0 && v <= b[i-1] {'))
mutate("桶写成空列表要启动失败", "xmetric/config.go", "./xmetric", "TestNew|TestInitXMetric",
       swap('\tif len(b) == 0 {\n', '\tif false {\n'))
mutate("Namespace 必须是合法的指标名前缀", "xmetric/config.go", "./xmetric", "TestNew",
       swap('if c.Namespace != "" && !nameRE.MatchString(c.Namespace) {', 'if false {'))
# le 通过启动、第一次 HistogramObserve 在业务请求里 panic；撞上框架指标的变量标签，
# 那组指标注册失败、只打一条错误日志
mutate("常量标签名不能是保留的标签名", "xmetric/config.go", "./xmetric", "TestNew",
       swap('if owner, ok := reservedLabels[k]; ok {', 'if owner, ok := reservedLabels[k]; false && ok {'))
# Config{} 的桶是 nil，原样交给 Prometheus 就是它的 DefBuckets 而不是本包的默认值
mutate("没给桶时用本包的默认值", "xmetric/xmetric.go", "./xmetric", "TestNew", swap('\tcfg = cfg.withDefaults()\n', ''))
# client_golang 现成的 collector 没有 Opts 可填，直接注册在 Registry 上就一个常量标签都不带：
# 按 env 过滤的看板查 go_goroutines{env="prod"} 什么都查不到
mutate("Go 运行时指标也带常量标签", "xmetric/xmetric.go", "./xmetric", "TestNew_运行时与进程指标也带常量标签",
       swap('withLabels.Register(promcollectors.NewGoCollector())', 'reg.Register(promcollectors.NewGoCollector())'))
mutate("进程指标也带常量标签", "xmetric/xmetric.go", "./xmetric", "TestNew_运行时与进程指标也带常量标签",
       swap('withLabels.Register(promcollectors.NewProcessCollector(', 'reg.Register(promcollectors.NewProcessCollector('))
# 不拦的话 version 撞上 go_info 自带的常量标签，报的是 client_golang 的 wrapping 错误，关掉 GoMetrics 又不报
mutate("常量标签名不能是 go_info 的 version", "xmetric/config.go", "./xmetric", "TestNew",
       swap('\t"version": "go_info in the Go runtime metrics",\n', ''))
# 挂上重试条件，resty 自己「不重试」的判断就作废了：200 + 坏 JSON 会被同一个 GET 重发
mutate("只重试传输层的错", "xhttp/xhttp.go", "./xhttp", "TestRetry",
       swap('\tif !isTransportError(err) {\n', '\tif err == nil {\n'))
mutate("关闭时清掉空闲连接", "xhttp/xhttp.go", "./xhttp", "TestNew",
       swap('\treturn client, &clientCloser{pool: pool}, nil','\treturn client, &clientCloser{pool: traced(cfg, pool)}, nil'))
mutate("重试耗时算整次逻辑请求", "xhttp/metric.go", "./xhttp", "TestMetric",
       swap('elapsed(resp.Request, resp.Time())','resp.Time()'))
# 连接信息交给 pgx 解。变异换回一个按空白切的解法：密码里的 host=… 就被读成地址
mutate("DSN 里的密码不进日志", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG密码片段",
       swap('\t\tAddr:         net.JoinHostPort(pc.Host, strconv.Itoa(int(pc.Port))),\n',
     '\t\tAddr:         net.JoinHostPort(naiveKV(dsn)["host"], strconv.Itoa(int(pc.Port))),\n'),
       swap('// seconds 向上取整为整秒',
     'func naiveKV(dsn string) map[string]string {\n\tall := map[string]string{}\n\tfor _, tok := range strings.Fields(dsn) {\n\t\tif k, v, ok := strings.Cut(tok, "="); ok {\n\t\t\tall[k] = v\n\t\t}\n\t}\n\treturn all\n}\n\n// seconds 向上取整为整秒'))
# pgx 同一个 key 取最后一次：默认值追加在后面的话，使用者显式写的值
# （包括 connect_timeout = 10 这种写法）就被盖掉了
mutate("DSN 里已写的 key 不被默认值盖掉", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG",
       swap('\treturn strings.Join(pairs, " ") + " " + dsn\n', '\treturn dsn + " " + strings.Join(pairs, " ")\n'))
mutate("垫在前面的参数不粘到 DSN 的第一项上", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG",
       swap('\treturn strings.Join(pairs, " ") + " " + dsn\n', '\treturn strings.Join(pairs, " ") + dsn\n'))
# gorm 另用正则取 DSN 里第一处时区，垫在前面的 Params 时区会抢在使用者写的前面
mutate("DSN 里写了时区就不垫时区", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_DSN里写了时区",
       swap('\tskipTimeZone := gormTimeZone.MatchString(dsn)\n', '\tskipTimeZone := false\n'))
mutate("首次建连受 ctx 管", "xgorm/xgorm.go", "./xgorm", "TestNew",
       swap('gorm.Config{DisableAutomaticPing: true}','gorm.Config{}'))
# 驱动的初始化查询挪到 Ready 里，就是为了受 ctx 管、跟着重试。调用点不接的话，
# 那次查询就没人做了
mutate("方言的 Ready 在建连探测里执行", "xgorm/xgorm.go", "./xgorm", "TestNew",
       swap('xclient.Probe(ctx, policy, probe(pool, readyOf(dialect, db)))', 'xclient.Probe(ctx, policy, probe(pool, nil))'))
# connect_timeout 管的是整个建连（TCP、TLS、认证），预算比它短就会
# 在 pgx 自己放弃之前把一次合法的慢握手判成超时
mutate("PG 的探测预算盖住 connect_timeout", "xgorm/dsn.go", "./xgorm", "TestProbeTimeout",
       swap('\treturn cmp.Or(connect, ceilSeconds(dial)) + dial\n', '\treturn dial + 0*cmp.Or(connect, ceilSeconds(dial))\n'))
# 配置里的超时只是默认值。DSN 里写了更长的，驱动就等那么久，预算还按配置算的话
# 会在驱动放弃之前把一次慢但合法的建连判超时。打在读出超时的地方和用它的地方
# 打在读出预算的调用点上（方言填的 ProbeTimeout）和各方言算预算的那一行
mutate("探测预算按 DSN 里写的超时放宽", "xgorm/xgorm.go", "./xgorm", "TestProbeTimeout",
       swap('\treturn cmp.Or(info.ProbeTimeout, 2*cfg.DialTimeout, fallbackPingTimeout)\n', '\treturn cmp.Or(2*cfg.DialTimeout, fallbackPingTimeout)\n'))
mutate("PG 的探测预算用 DSN 里的 connect_timeout", "xgorm/dsn.go", "./xgorm", "TestProbeTimeout_DSN",
       swap('ProbeTimeout: postgresProbeTimeout(pc.ConnectTimeout, c.DialTimeout),', 'ProbeTimeout: postgresProbeTimeout(0, c.DialTimeout),'))
mutate("MySQL 的探测预算用 DSN 里的超时", "xgorm/dsn.go", "./xgorm", "TestProbeTimeout_DSN",
       swap('\t\tProbeTimeout: cfg.Timeout + cfg.ReadTimeout,\n', '\t\tProbeTimeout: c.DialTimeout + c.MySQL.ReadTimeout,\n'))
mutate("ClickHouse 的探测预算用 DSN 里的 dial_timeout", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve_DSN里写了的",
       swap('\t\tProbeTimeout: 2 * opts.DialTimeout,\n', '\t\tProbeTimeout: 2*c.DialTimeout + 0*opts.DialTimeout,\n'))
# 驱动在 Initialize 里用 context.Background() 查版本：ctx 取消了也要等满
# dial_timeout，失败了一次重试都没有
mutate("ClickHouse 首次建连受 ctx 管也会重试", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestNew",
       swap('SkipInitializeWithVersion: true', 'SkipInitializeWithVersion: false'))
mutate("ClickHouse 仍然查版本", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestRegister",
       swap('\tReady:      probeVersion,\n', ''))
# 原样透传的话，驱动建连时的解析错误会连同整串 DSN、包括明文密码一起进日志
mutate("ClickHouse 不是 URL 的 DSN 被拒绝", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve",
       swap('\t\treturn "", xgorm.ConnInfo{}, errNotURL\n', '\t\treturn c.DSN, xgorm.ConnInfo{Driver: string(Driver)}, nil\n'))
# 多主机 DSN 的 URL Host 是整串 "h1:9000,h2:9000"：当成地址的话 Span 的 server.address 是整串、
# 没有 server.port，建连日志里也不是一个「主机:端口」
mutate("ClickHouse 多主机 DSN 记第一个主机", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve_多主机",
       swap('\t\tAddr:         opts.Addr[0],\n', '\t\tAddr:         u.Host,\n'))
mutate("ClickHouse 驱动的解析错误不回显 DSN", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve",
       swap('parseDSN(dsn, c.TLS.Enable)\n\tif err != nil {\n\t\treturn "", xgorm.ConnInfo{}, errMalformedDSN',
     'parseDSN(dsn, c.TLS.Enable)\n\tif err != nil {\n\t\treturn "", xgorm.ConnInfo{}, err'))
mutate("Redis 命令遵守请求 deadline", "xredis/xredis.go", "./xredis", "TestNew",
       swap('\t\tContextTimeoutEnabled: true,\n',''))
# go-redis 默认每次建连内部重拨 5 次、间隔 100ms：主机宕机时一条命令实测 11.7s，
# 远超文档那条式子推出来的 7s
mutate("Redis 一次建连只拨一次号", "xredis/xredis.go", "./xredis", "TestNew_Redis挂了时",
       swap('\t\tDialerRetries: 1,\n', ''))
# 打在调用点上：绕开 probe 直接 Ping，启动期间的退出信号要等到 ReadTimeout 才生效
mutate("Redis 建连探测收到退出信号当场放弃", "xredis/xredis.go", "./xredis", "TestNew_启动时收到退出信号",
       swap('xclient.Probe(ctx, probePolicy(cfg), probe(client))',
            'xclient.Probe(ctx, probePolicy(cfg), func(ctx context.Context) error { return client.Ping(ctx).Err() })'))
mutate("Redis 建连探测取消时不等这次读", "xredis/xredis.go", "./xredis", "TestNew_启动时收到退出信号",
       swap('\t\t\tif errors.Is(ctx.Err(), context.Canceled) {\n', '\t\t\tif false && errors.Is(ctx.Err(), context.Canceled) {\n'))
mutate("Redis 认证失败不重试", "xredis/xredis.go", "./xredis", "TestNew_认证失败",
       swap('\t\tAuthFailed: redis.IsAuthError,\n', ''))
mutate("Redis 认证失败不说成连不上", "xredis/xredis.go", "./xredis", "TestNew_认证失败",
       swap('\t\tif redis.IsAuthError(err) {\n\t\t\treturn nil, nil, xerror.Newf', '\t\tif false {\n\t\t\treturn nil, nil, xerror.Newf'))
# 不接的话 go-redis 往 stderr 写纯文本，不是 JSON、不带 trace_id
mutate("go-redis 自己的日志进 slog", "xredis/xredis.go", "./xredis", "TestGoRedis自己的日志",
       swap('\tredis.SetLogger(slogLogger{})\n', ''))
mutate("go-redis 的日志带着调用方的 ctx", "xredis/xredis.go", "./xredis", "TestGoRedis自己的日志",
       swap('slog.WarnContext(ctx, "xredis go-redis log"', 'slog.WarnContext(context.Background(), "xredis go-redis log"'))
# GORM 只在 Logger 实现了 ParamsFilter 时才不把参数代进 SQL，
# redisotel 默认把整条命令连同参数写进 Span：两处都是凭证出去的口子
mutate("SQL 日志里没有参数值", "xgorm/logger.go", "./xgorm", "TestLogger",
       swap('func (l *gormLogger) ParamsFilter(', 'func (l *gormLogger) paramsFilter('))
# Scan 借用的 Recorder 不问实例 Logger 的 ParamsFilter，只认进程级的 RecorderParamsFilter：
# 打在 init 里接它的那一行上
mutate("Scan 的 SQL 日志里没有参数值", "xgorm/xgorm.go", "./xgorm", "TestLogger_Scan",
       swap('\tlogger.RecorderParamsFilter = withoutParams\n', ''))
# PG 方言的 Explain 没参数可代时把 $1 留成 $1$：日志里的语句和发出去的对不上。
# 打在记 SQL 的调用点上，和判断方言占位符的那一处
mutate("PG 的 SQL 日志就是发出去的那条", "xgorm/logger.go", "./xgorm", "TestLogger",
       swap('\t\tsql, rows := l.statement(fc)\n\t\tslog.InfoContext', '\t\tsql, rows := fc()\n\t\tslog.InfoContext'))
mutate("认得出方言会改写 $N 占位符", "xgorm/logger.go", "./xgorm", "TestLogger",
       swap('numbered:       d.Explain("$1") == "$1$",', 'numbered:       false,'))
# 密码错了重试也是错，还多等两轮退避；报成 cannot reach 会让人先去查网络
# 同一个机制（xclient.Probe）两处都要打：Probe 里认出来就不再试，xgorm 把方言的判断交给它
mutate("认证失败不重试", "internal/xclient/probe.go", ".", "TestProbe_认证失败",
       swap('\t\t\treturn xutil.Permanent(err)\n', '\t\t\treturn err\n'))
mutate("xgorm 认证失败不重试", "xgorm/xgorm.go", "./xgorm", "TestNew_认证失败|TestNew_MySQL认证失败",
       swap('\t\tAuthFailed: d.authFailed,\n', ''))
mutate("认证失败报的是认证失败", "xgorm/xgorm.go", "./xgorm", "TestNew_认证失败",
       swap('\t\tif dialect.authFailed(err) {\n\t\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)\n'
            '\t\t}\n\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "cannot reach',
            '\t\tif false {\n\t\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)\n'
            '\t\t}\n\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "cannot reach'))
# 注册进来的方言要是在 Initialize 里建连，认证错误就在 gorm.Open 里出来，走不到 ping：
# 只在 ping 那条路上认的话，密码错报的是 open … failed。打在 gorm.Open 的调用点上
# （内置的 MySQL 原先就是这样，查版本挪进 Ready 之后走的是 ping 那条路）
mutate("MySQL 认证失败在 gorm.Open 那条路上也报认证失败", "xgorm/xgorm.go", "./xgorm", "TestNew_MySQL认证失败",
       swap('\t\tif dialect.authFailed(err) {\n\t\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)\n'
            '\t\t}\n\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "open',
            '\t\tif false {\n\t\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "authentication to %s failed: %w", info.Addr, err)\n'
            '\t\t}\n\t\treturn nil, nil, xerror.Newf("xgorm", "connect", "open'))
mutate("认得出 MySQL 的 1045 / 1044", "xgorm/mysql.go", "./xgorm", "TestNew_MySQL认证失败|TestDialect_内置方言",
       swap('return errors.As(err, &myErr) && (myErr.Number == mysqlAccessDenied || myErr.Number == mysqlDBAccessDenied)',
            'return errors.As(err, &myErr) && false'))
# MySQL 的 Dialector 在 Initialize 里用 context.Background() 查版本：那是第一次建连，
# 失败了 gorm.Open 直接返回，三次重试一次都没轮上，退出信号也管不到
mutate("MySQL 首次建连受 ctx 管也会重试", "xgorm/mysql.go", "./xgorm", "TestNew_MySQL对端不回话|TestMySQL方言",
       swap('SkipInitializeWithVersion: true})', 'SkipInitializeWithVersion: false})'))
mutate("MySQL 仍然查版本", "xgorm/dialect.go", "./xgorm", "TestMySQL方言|TestProbeMySQLVersion",
       swap(', Ready: probeMySQLVersion,\n', ',\n'))
mutate("MySQL 查版本受 ctx 管", "xgorm/mysql.go", "./xgorm", "TestProbeMySQLVersion",
       swap('db.ConnPool.QueryRowContext(ctx, "SELECT VERSION()")', 'db.ConnPool.QueryRowContext(context.Background(), "SELECT VERSION()")'))
# 版本号不设进开关的话，老版本 / MariaDB 上迁移生成它不认的 DDL
mutate("MySQL 的版本号设进开关", "xgorm/mysql.go", "./xgorm", "TestProbeMySQLVersion|TestApplyMySQLVersion",
       swap('\treturn applyMySQLVersion(db, d.Config, v)\n', '\t_, _ = d, v\n\treturn nil\n'))
mutate("MariaDB 10.5+ 的增删改带 RETURNING", "xgorm/mysql.go", "./xgorm", "TestApplyMySQLVersion",
       swap('\t\treturn enableReturning(db)\n', '\t\treturn nil\n'))
# 打在调用点上：探测不用这一轮的 ctx，卡在握手读上时退出信号要等满 ReadTimeout
mutate("MySQL 建连探测收到退出信号当场放弃", "xgorm/xgorm.go", "./xgorm", "TestNew_MySQL启动期间取消",
       swap('err := pool.PingContext(ctx)', 'err := pool.PingContext(context.Background())'))
# 认证失败的识别住在方言里：内置方言不接上的话，核心里再没有别处认得出来
mutate("内置 PG 方言认得出认证失败", "xgorm/dialect.go", "./xgorm", "TestDialect_内置方言",
       swap('\t\tAuthFailed: postgresAuthFailed, ErrorCode: postgresErrorCode,\n', '\t\tErrorCode: postgresErrorCode,\n'))
mutate("内置 MySQL 方言认得出认证失败", "xgorm/dialect.go", "./xgorm", "TestDialect_内置方言",
       swap('\t\tAuthFailed: mysqlAuthFailed, ErrorCode: mysqlErrorCode,\n', '\t\tErrorCode: mysqlErrorCode,\n'))
mutate("认得出 PG 的 28 类", "xgorm/dsn.go", "./xgorm", "TestDialect_内置方言|TestNew_认证失败",
       swap('return strings.HasPrefix(postgresErrorCode(err), "28")', 'return postgresErrorCode(err) == "28P01"'))
mutate("ClickHouse 认证失败不重试", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestNew_认证失败|TestDialect_",
       swap('\tAuthFailed: authFailed,\n', ''))
mutate("ClickHouse 认得出老版本的认证错误码", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestDialect_",
       swap('\tchproto.ErrWrongPassword,        // 193\n', ''))
# 服务端错误原文里带着参数值（MySQL 1062 的 Duplicate entry 'a@b.com'）：
# 日志和 Span 两个出口都要打在调用点上，外加方言接上错误码的那一处
mutate("SQL 日志不记服务端错误原文", "xgorm/logger.go", "./xgorm", "TestLogger_服务端报错",
       swap('attrs(sql, rows, elapsed, l.errorAttrs(err)...)', 'attrs(sql, rows, elapsed, "error", err)'))
mutate("Span 不记服务端错误原文", "xgorm/trace.go", "./xgorm", "TestSpan_服务端报错",
       swap('\t\t\trecordError(span, d, db.Error)\n', '\t\t\tspan.RecordError(db.Error)\n\t\t\tspan.SetStatus(codes.Error, db.Error.Error())\n'))
mutate("Span 服务端报错时不调 RecordError", "xgorm/trace.go", "./xgorm", "TestSpan_服务端报错",
       swap('\t\tspan.SetAttributes(semconv.DBResponseStatusCode(code), semconv.ErrorTypeKey.String(code))\n',
            '\t\tspan.SetAttributes(semconv.DBResponseStatusCode(code), semconv.ErrorTypeKey.String(code))\n\t\tspan.RecordError(err)\n'))
mutate("MySQL 方言认得出服务端错误码", "xgorm/dialect.go", "./xgorm", "TestLogger_服务端报错|TestSpan_服务端报错",
       swap('AuthFailed: mysqlAuthFailed, ErrorCode: mysqlErrorCode,', 'AuthFailed: mysqlAuthFailed,'))
mutate("PG 方言认得出服务端错误码", "xgorm/dialect.go", "./xgorm", "TestLogger_服务端报错|TestSpan_服务端报错",
       swap('AuthFailed: postgresAuthFailed, ErrorCode: postgresErrorCode,', 'AuthFailed: postgresAuthFailed,'))
mutate("ClickHouse 方言认得出服务端错误码", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestDialect_",
       swap('\tErrorCode:  errorCode,\n', ''))
# 驱动默认 parseTime=false：DATETIME 扫不进 time.Time
mutate("MySQL 没写 parseTime 时补成 true", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_MySQL没写parseTime",
       swap('\tif !mysqlParamSet(c.DSN, "parseTime") {\n', '\tif false && !mysqlParamSet(c.DSN, "parseTime") {\n'))
mutate("DSN 里写了 parseTime 以 DSN 为准", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_MySQL没写parseTime",
       swap('\tif !mysqlParamSet(c.DSN, "parseTime") {\n', '\tif true {\n'))
mutate("密码里的 parseTime 骗不过它", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_MySQL没写parseTime",
       swap("\ti := strings.LastIndexByte(dsn, '/')\n", "\ti := strings.IndexByte(dsn, '/')\n"))
# 配置在读的时候就校验：负的时长底下每一处都静默变成「不限」
mutate("负的时长被拒", "xgorm/config.go", "./xgorm", "TestValidate",
       swap('\t\tif d.val < 0 {\n', '\t\tif false && d.val < 0 {\n'))
mutate("New 也校验配置", "xgorm/xgorm.go", "./xgorm", "TestNew_配置有误",
       swap('\tif err := cfg.Validate(); err != nil {\n', '\tif err := error(nil); err != nil {\n'))
mutate("多实例的 Validate 点名实例", "xgorm/config.go", "./xgorm", "TestConfig_Validate报出",
       swap('errs = append(errs, fmt.Errorf("Clients.%s: %w", name, err))', 'errs = append(errs, err)'))
mutate("实例的 Validate 错误带着文件和行号", "internal/config/clients.go", ".", "TestUnmarshalClients_每个实例都调一次Validate",
       swap('\t\t\treturn c, fmt.Errorf("%s: %w", newChecker().at(n), err)\n', '\t\t\treturn c, err\n'))
# 建连日志要写是哪个实例：打在 build 往 open 传名字的调用点上
mutate("建连日志写着实例名", "xgorm/xgorm.go", "./xgorm", "TestInstall_建连日志",
       swap('open(ctx, c.name, c.ClientConfig)', 'open(ctx, "", c.ClientConfig)'))
# OTel 数据库语义约定的名字：旧名字换回来，看板和采集规则就对不上
mutate("Span 用语义约定的 db.query.text", "xgorm/trace.go", "./xgorm", "TestSpan_带上连接信息",
       swap('span.SetAttributes(semconv.DBQueryText(sql),', 'span.SetAttributes(attribute.String("db.statement", sql),'))
mutate("Span 的 db.system.name 是 postgresql", "xgorm/trace.go", "./xgorm", "TestConnAttrs",
       swap('\t\treturn "postgresql"\n', '\t\treturn driver\n'))
mutate("Span 的地址拆成主机和端口", "xgorm/trace.go", "./xgorm", "TestSpan_带上连接信息|TestConnAttrs",
       swap('\tout = append(out, semconv.ServerAddress(host))\n', '\tout = append(out, semconv.ServerAddress(info.Addr+host[:0]))\n'))
mutate("db.operation.name 取语句的第一个关键字", "xgorm/trace.go", "./xgorm", "TestSpan_带上连接信息",
       swap('\tif op := operationName(sql); op != "" {\n', '\tif op := ""; op != "" {\n'))
mutate("连接池指标叫 db_pool_*", "xgorm/metric.go", "./xgorm", "TestPoolCollector|TestInstall",
       swap('{"db_pool_open", ', '{"db_connections_open", '))

# 不接的话 go-sql-driver 往 stderr 写 [mysql] … 纯文本，不是 JSON
mutate("go-sql-driver 自己的日志进 slog", "xgorm/xgorm.go", "./xgorm", "TestNew_MySQL驱动自己的日志",
       swap('\t_ = mysqldriver.SetLogger(mysqlDriverLogger{})', '\t_ = mysqldriver.SetLogger(nil) // 只在参数为 nil 时报错，于是什么都没设'))
mutate("Redis 命令参数不进 Span", "xredis/xredis.go", "./xredis", "TestTrace",
       swap('redisotel.InstrumentTracing(client, redisotel.WithDBStatement(false))', 'redisotel.InstrumentTracing(client)'))
# collector 是进程级的一个、抓取时遍历全部实例，不看实例自己的开关，
# Metric: false 就是一句空话
mutate("Metric 关掉的数据库实例不导出", "xgorm/xgorm.go", "./xgorm", "TestInstall", swap('if !ok || !inst.metric {', 'if !ok {'))
mutate("Metric 关掉的 Redis 实例不导出", "xredis/xredis.go", "./xredis", "TestInstall", swap('ok && inst.metric {', 'ok {'))
# 进程级的「只挂一次」会把 xmetric 重装之后的那次挡掉：新 Registry 上
# 没有连接池 collector，第二轮生命周期里这组指标一个都导不出去
mutate("xmetric 重装后数据库连接池指标照样导出", "xgorm/xgorm.go", "./xgorm", "TestInstall",
       swap('func installPoolMetrics() {\n\tif _, err := xmetric.RegisterAs(',
     'var poolInstalled bool\n\nfunc installPoolMetrics() {\n\tif poolInstalled {\n\t\treturn\n\t}\n\tpoolInstalled = true\n\tif _, err := xmetric.RegisterAs('))
mutate("xmetric 重装后 Redis 连接池指标照样导出", "xredis/xredis.go", "./xredis", "TestInstall",
       swap('func installPoolMetrics() {\n\tif _, err := xmetric.RegisterAs(',
     'var poolInstalled bool\n\nfunc installPoolMetrics() {\n\tif poolInstalled {\n\t\treturn\n\t}\n\tpoolInstalled = true\n\tif _, err := xmetric.RegisterAs('))
mutate("MaxCost 就是能存多少条", "xcache/xcache.go", "./xcache", "TestNew",
       swap('IgnoreInternalCost: true,','IgnoreInternalCost: false,'))
# ristretto 的 Close 与并发读写一起跑会 send on closed channel，
# 而拿着原生 *Cache 的调用方框架拦不住
mutate("关缓存时有人在读写也不崩", "xcache/xcache.go", "./xcache", "TestClose", swap('closerFunc(c.Clear)', 'closerFunc(c.Close)'))
mutate("缓存的 DefaultTTL 不能为负", "xcache/config.go", "./xcache", "TestValidate",
       swap('\tif c.DefaultTTL < 0 {', '\tif false {'))
# 名字写错时静默返回 0，而 0 在 ristretto 里是「永不过期」
mutate("DefaultTTL 取不到实例就 panic", "xcache/xcache.go", "./xcache", "TestDefaultTTL",
       swap('{ return reg.Get(name...).ttl }', '{ inst, _ := reg.Lookup(name...); return inst.ttl }'))
# 读配置时就校验靠的是实例类型实现了 Validate：换成一个没有方法的同构类型，
# 配错的值就一路放到 New 才报，报错里也不再有文件和实例名
mutate("XCache 块在读配置时就校验", "xcache/config.go", "./xcache", "TestConfig_不合法的值在读配置时就失败",
       swap('clients, err := xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)\n\treturn Config{Clients: clients}, err',
            'type raw ClientConfig\n\tm, err := xconfig.UnmarshalClients(ConfigKey, func() raw { return raw(DefaultClientConfig()) })\n\tclients := map[string]ClientConfig{}\n\tfor k, v := range m {\n\t\tclients[k] = ClientConfig(v)\n\t}\n\treturn Config{Clients: clients}, err'))
# ristretto 默认不计数，Metrics 为 nil 时每个计数都是 0：Metric: true 也什么都看不到
mutate("缓存的 Metric 开关传给 ristretto", "xcache/xcache.go", "./xcache", "TestNew_Metric开关|TestCacheCollector",
       swap('Metrics: cfg.Metric,', 'Metrics: false,'))
mutate("Metric 关掉的缓存实例不导出", "xcache/xcache.go", "./xcache", "TestInstall_只导出开了Metric", swap('ok && inst.metric {', 'ok {'))
mutate("xmetric 重装后缓存指标照样导出", "xcache/xcache.go", "./xcache", "TestInstall_只导出开了Metric",
       swap('func installMetrics() {\n\tif _, err := xmetric.RegisterAs(',
     'var installed bool\n\nfunc installMetrics() {\n\tif installed {\n\t\treturn\n\t}\n\tinstalled = true\n\tif _, err := xmetric.RegisterAs('))
mutate("缓存实例的日志带着名字", "xcache/xcache.go", "./xcache", "TestInstall_每个实例的日志带着名字",
       swap('"xcache created", "name", c.name,', '"xcache created", "name", "",'))
# 调用点：字段在 Config 里、Validate 里都有，没赋给 Transport 就是一句空话
mutate("MaxConnsPerHost 传给连接池", "xhttp/xhttp.go", "./xhttp", "TestNew_每host连接数上限生效",
       swap('\tt.MaxConnsPerHost = cfg.MaxConnsPerHost\n', ''))
mutate("MaxConnsPerHost 为负要被拦住", "xhttp/config.go", "./xhttp", "TestValidate",
       swap(' || c.MaxConnsPerHost < 0 {', ' {'))
mutate("XRedis 块在读配置时就校验", "xredis/config.go", "./xredis", "TestConfig_不合法的值在读配置时就失败",
       swap('clients, err := xconfig.UnmarshalClients(ConfigKey, DefaultClientConfig)\n\treturn Config{Clients: clients}, err',
            'type raw ClientConfig\n\tm, err := xconfig.UnmarshalClients(ConfigKey, func() raw { return raw(DefaultClientConfig()) })\n\tclients := map[string]ClientConfig{}\n\tfor k, v := range m {\n\t\tclients[k] = ClientConfig(v)\n\t}\n\treturn Config{Clients: clients}, err'))
mutate("直接调 xredis.New 也校验", "xredis/xredis.go", "./xredis", "TestNew_配置有误时不建连|TestNew_TLS文件",
       swap('\tif err := cfg.Validate(); err != nil {', '\tif err := cfg.Validate(); false && err != nil {'))
# go-redis 把 ReadTimeout -1 当成「不限时」：一个减号就静默关掉超时保护
mutate("Redis 负的时长要被拦住", "xredis/config.go", "./xredis", "TestValidate|TestConfig_不合法的值在读配置时就失败",
       swap('\t\tif d.val < 0 {\n\t\t\treturn fmt.Errorf("%s must not be negative', '\t\tif false {\n\t\t\treturn fmt.Errorf("%s must not be negative'))
mutate("Redis 负的连接数要被拦住", "xredis/config.go", "./xredis", "TestValidate",
       swap('\t\tif n.val < 0 {', '\t\tif false {'))
mutate("Redis MaxRetries 只收 -1 这一个负数", "xredis/config.go", "./xredis", "TestValidate",
       swap('if c.MaxRetries < -1 {', 'if false {'))
mutate("Redis 退避只收 -1ns 这一个负数", "xredis/config.go", "./xredis", "TestValidate",
       swap('if d.val < 0 && d.val != -1 {', 'if false {'))
# TLS 块（xtls.Config）一处定义、各模块共用：规则打在 xtls 里，
# 「这个模块真的用了它」打在每个模块的调用点上——函数本身对，调用点绕开了照样是 bug
# 写了 CAFile 却忘了 Enable，照明文连过去比报错更糟
mutate("没开 TLS 却写了 TLS 字段要失败", "xtls/xtls.go", ".", "TestValidate_说不通的组合|TestBuild_没开却写了",
       swap('\t\tif c != (Config{}) {', '\t\tif false {'))
mutate("TLS 客户端证书和私钥要成对", "xtls/xtls.go", ".", "TestValidate_说不通的组合",
       swap('if (c.CertFile == "") != (c.KeyFile == "") {', 'if false {'))
mutate("直接调 Build 也先校验", "xtls/xtls.go", ".", "TestBuild_没开却写了",
       swap('\tif err := c.Validate(); err != nil {\n\t\treturn nil, err\n\t}\n\tif !c.Enable', '\tif !c.Enable'))
mutate("TLS 用上 CAFile", "xtls/xtls.go", ".", "TestBuild_CAFile",
       swap('\t\tcfg.RootCAs = pool\n', '\t\t_ = pool\n'))
mutate("TLS 带上客户端证书", "xtls/xtls.go", ".", "TestBuild_客户端证书",
       swap('\t\tcfg.Certificates = []tls.Certificate{cert}\n', '\t\t_ = cert\n'))
mutate("TLS 的 ServerName 传下去", "xtls/xtls.go", ".", "TestBuild_CAFile|TestBuild_最低",
       swap('ServerName: c.ServerName}', '}'))
mutate("TLS 最低 1.2", "xtls/xtls.go", ".", "TestBuild_最低",
       swap('MinVersion: tls.VersionTLS12, ', ''))
# 证书被拒和密码错一样：再试还是同一张证书。PG / MySQL / Redis 的建连探测都走这里
mutate("证书被拒不重试", "internal/xclient/probe.go", ".", "TestProbe",
       swap('(tlsRejected(err) || p.AuthFailed != nil && p.AuthFailed(err))', '(p.AuthFailed != nil && p.AuthFailed(err))'))
mutate("对端的证书告警也算证书被拒", "internal/xclient/probe.go", ".", "TestProbe_证书被拒",
       swap('if errors.As(err, &op) && op.Op == "remote error" && op.Err != nil {', 'if false {'))
mutate("Redis 校验 TLS 块", "xredis/config.go", "./xredis", "TestValidate",
       swap('\treturn c.TLS.Validate()\n}', '\treturn nil\n}'))
mutate("Redis 的 TLS 配置传给 go-redis", "xredis/xredis.go", "./xredis", "TestNew_TLS",
       swap('tlsCfg, err := cfg.TLS.Build()', '_, err := cfg.TLS.Build()'), swap('TLSConfig: tlsCfg,', 'TLSConfig: nil,'))
mutate("XGorm 校验 TLS 块", "xgorm/config.go", "./xgorm", "TestValidate_TLS块|TestConfig_TLS块",
       swap('\tif err := c.TLS.Validate(); err != nil {\n\t\treturn err\n\t}\n', ''))
mutate("方言不收 TLS 块时配了要失败", "xgorm/config.go", "./xgorm", "TestValidate_TLS块",
       swap('if c.TLS.Enable && d.OpenTLS == nil {', 'if c.TLS.Enable && d.OpenTLS == nil && false {'))
mutate("开了 TLS 块走 OpenTLS", "xgorm/dialect.go", "./xgorm", "TestNew_PG_TLS|TestNew_MySQL_TLS",
       swap('\tif cfg == nil {\n\t\treturn d.Open(dsn), nil\n\t}', '\tif true {\n\t\treturn d.Open(dsn), nil\n\t}'))
# pgx 默认的 prefer 不校验证书、服务端不肯 TLS 就改走明文
mutate("PG 建连时换上 TLS 块的配置", "xgorm/tls.go", "./xgorm", "TestNew_PG_TLS",
       swap('\t\t\tusePostgresTLS(&cc.Config, cfg)\n', ''))
mutate("PG 开了 TLS 块不退回明文", "xgorm/tls.go", "./xgorm", "TestNew_PG_TLS块开着时|TestUsePostgresTLS",
       swap('\tc.Fallbacks = hosts[1:]\n', ''))
mutate("PG 没配 ServerName 时按主机名比对", "xgorm/tls.go", "./xgorm", "TestNew_PG_TLS块生效|TestUsePostgresTLS",
       swap('\t\t\tif tc.ServerName == "" {\n\t\t\t\ttc.ServerName = h.Host\n\t\t\t}\n', ''))
mutate("PG TLS 块和 DSN 里的 ssl 参数不能同时写", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_TLS块和",
       swap('\t\tif err := checkPostgresTLSParams(dsn); err != nil {', '\t\tif err := checkPostgresTLSParams(dsn); false && err != nil {'))
mutate("PG 冲突查的是补完 Params 之后的 DSN", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_TLS块和",
       swap('checkPostgresTLSParams(dsn)', 'checkPostgresTLSParams(c.DSN)'))
mutate("PG 只有 ssl 开头的参数算冲突", "xgorm/tls.go", "./xgorm", "TestResolveDSN_PG_TLS块和",
       swap('if strings.HasPrefix(key, "ssl") {', 'if strings.HasPrefix(key, "") {'))
mutate("PG TLS 块不收 Unix socket", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_PG_TLS块不能配Unix",
       swap('\t\tif err := checkPostgresTCP(pc); err != nil {', '\t\tif err := checkPostgresTCP(pc); false && err != nil {'))
mutate("MySQL TLS 块和 DSN 里的 tls 不能同时写", "xgorm/dsn.go", "./xgorm", "TestResolveDSN_MySQL_TLS块",
       swap('\t\tif err := checkMySQLTLS(c.DSN, cfg); err != nil {', '\t\tif err := checkMySQLTLS(c.DSN, cfg); false && err != nil {'))
mutate("MySQL 连接配置带上 TLS 块", "xgorm/tls.go", "./xgorm", "TestNew_MySQL_TLS",
       swap('\tdc.TLS = cfg.Clone()\n', ''))
# ClickHouse 的 TLS 块：打在注册的方言、resolve 与 openTLS 的调用点上
mutate("ClickHouse 收 TLS 块", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestRegister_OpenTLS|TestNew_TLS块",
       swap('\tOpenTLS:    openTLS,\n', ''))
mutate("ClickHouse 连接配置带上 TLS 块", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestNew_TLS块",
       swap('\topts.TLS = cfg.Clone()\n', ''))
mutate("ClickHouse TLS 块和 DSN 里的 TLS 参数不能同时写", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve_TLS块",
       swap('\t\tif err := checkTLS(u, q); err != nil {', '\t\tif err := checkTLS(u, q); false && err != nil {'))
# http:// 按解析时记下的 scheme 拼请求地址，手里有 *tls.Config 也发明文
mutate("ClickHouse http:// 配 TLS 块是配置错误", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestResolve_TLS块不收http",
       swap('if u.Scheme == "http" {', 'if false {'))
mutate("ClickHouse 开了 TLS 块时 https 不必写 secure", "xgorm/clickhouse/clickhouse.go", "./xgorm/clickhouse", "TestResolve_TLS块开着时https",
       swap('opts, err := parseDSN(dsn, c.TLS.Enable)', 'opts, err := chgo.ParseDSN(dsn)'))
mutate("ClickHouse openTLS 按 https 解 https", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestNew_TLS块生效",
       swap('opts, err := parseDSN(dsn, true)', 'opts, err := parseDSN(dsn, false)'))
mutate("ClickHouse https 补 secure 才记得住 scheme", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestParseDSN_https|TestNew_TLS块生效",
       swap('\t\tq.Set("secure", "true")\n', '\t\tq.Set("secure", "false")\n'))
# GORM 的驱动拿到 DSN 会另解一份，UpdateLocalTable 按它直连每台主机、不带 TLS 块
mutate("ClickHouse TLS 下不把 DSN 交给 GORM 的驱动", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestOpenTLS",
       swap('clickhouse.Config{Conn: chgo.OpenDB(opts),', 'clickhouse.Config{DSN: dsn, Conn: chgo.OpenDB(opts),'))
mutate("ClickHouse TLS 下同样关掉驱动自带的查版本", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestOpenTLS",
       swap('SkipInitializeWithVersion: true', 'SkipInitializeWithVersion: false'))
mutate("ClickHouse openTLS 的解析错误不回显 DSN", "xgorm/clickhouse/tls.go", "./xgorm/clickhouse", "TestOpenTLS",
       swap('\t\treturn nil, errMalformedDSN // 不回传驱动的错误', '\t\treturn nil, err // 不回传驱动的错误'))
mutate("XHttp 校验 TLS 块", "xhttp/config.go", "./xhttp", "TestValidate_TLS块",
       swap('\treturn c.TLS.Validate()\n}', '\treturn nil\n}'))
mutate("XHttp 的 TLS 块交给连接池", "xhttp/xhttp.go", "./xhttp", "TestNew_TLS",
       swap('\t\tt.TLSClientConfig = tlsCfg\n', ''))
mutate("XGin 服务端 TLS 设置交给 http.Server", "xgin/xgin.go", "./xgin", "TestStart_ClientCAFile|TestStart_MinVersion",
       swap('\tsrv.TLSConfig = tlsCfg', '\t_ = tlsCfg'))
mutate("XGin ClientCAFile 开双向认证", "xgin/config.go", "./xgin", "TestStart_ClientCAFile开双向认证",
       swap('\t\tcfg.ClientAuth = tls.RequireAndVerifyClientCert\n', ''))
mutate("XGin MinVersion 生效", "xgin/config.go", "./xgin", "TestStart_MinVersion",
       swap('cfg := &tls.Config{MinVersion: tlsVersions[c.MinVersion]}', 'cfg := &tls.Config{}'))
# 以为开了双向认证，实际是谁都能连的明文
mutate("XGin ClientCAFile 要和证书一起配", "xgin/config.go", "./xgin", "TestValidate_服务端TLS|TestConfig_服务端TLS",
       swap('if c.ClientCAFile != "" && !c.tlsEnabled() {', 'if false {'))
mutate("XGin MinVersion 只收 1.2 / 1.3", "xgin/config.go", "./xgin", "TestValidate_服务端TLS",
       swap('if _, ok := tlsVersions[c.MinVersion]; !ok {', 'if false {'))
# Redis 7.2 之前每条新连接一个报错的 CLIENT SETINFO Span
mutate("Redis 建连不发 CLIENT SETINFO", "xredis/xredis.go", "./xredis", "TestNew_不发CLIENT_SETINFO",
       swap('DisableIdentity: true,', 'DisableIdentity: false,'))
mutate("Redis 不开维护通知", "xredis/xredis.go", "./xredis", "TestNew_不发CLIENT_SETINFO",
       swap('Mode: maintnotifications.ModeDisabled}', 'Mode: maintnotifications.ModeAuto}'))
# 钩子挂在建连验证之前：启动时每次 Ping 尝试都是一个没有父 Span 的 ping
mutate("Redis 链路钩子在建连验证成功之后才挂", "xredis/xredis.go", "./xredis", "TestTrace_启动时的建连验证不开Span",
       swap('\tif err := xclient.Probe(ctx, probePolicy(cfg), probe(client)); err != nil {',
            '\tif cfg.Trace {\n\t\t_ = redisotel.InstrumentTracing(client, redisotel.WithDBStatement(false))\n\t}\n\tif err := xclient.Probe(ctx, probePolicy(cfg), probe(client)); err != nil {'))
mutate("xredis connected 日志带着实例名", "xredis/xredis.go", "./xredis", "TestInstall_日志写出实例名",
       swap('"xredis connected", "name", c.name,', '"xredis connected", "name", "",'))
mutate("Log 关掉时 GORM 不自己往标准输出写", "xgorm/xgorm.go", "./xgorm", "TestNew",
       swap('gormCfg.Logger = logger.Discard','gormCfg.Logger = logger.Default'))
# 建到一半失败时把半套实例发布出去，比一个都没有更糟：C() 取得到 a
# 取不到 b，而启动其实已经失败了
mutate("建失败时注册表保持原样", "internal/xclient/xclient.go", ".", "TestBuild",
       swap('''\t\tbuilt[name] = v
\t\tclosers = append(closers, closer)''', '''\t\tbuilt[name] = v
\t\tclosers = append(closers, closer)
\t\tr.state.Store(&snapshot[T]{items: built, phase: running})'''))
# 反过来的话，关到一半时 C() 还能取到正在被关闭的实例
mutate("关实例先摘再关", "internal/xclient/xclient.go", ".", "TestClose",
       swap('''\told := r.state.Swap(&snapshot[T]{items: map[string]T{}, phase: closed})
\treturn closeAll(r.module, old.closers)''', '''\terr := closeAll(r.module, r.state.Load().closers)
\tr.state.Store(&snapshot[T]{items: map[string]T{}, phase: closed})
\treturn err'''))
# 一个减号换来的静默故障：DialTimeout 为负时每次请求当场 i/o timeout，
# Timeout 为负反而被标准库当成「不限时」，超时保护整个消失
mutate("负的时长要被拦住", "xhttp/config.go", "./xhttp", "TestInitXHttp|TestValidate",
       swap('\t\tif d.val < 0 {', '\t\tif false {'))
mutate("XHttp 块在读配置时就校验", "xhttp/xhttp.go", "./xhttp", "TestInitXHttp",
       swap('xconfig.Unmarshal(ConfigKey, &c)', 'func() error { type raw Config; return xconfig.Unmarshal(ConfigKey, (*raw)(&c)) }()'))
mutate("直接调 xhttp.New 也校验", "xhttp/xhttp.go", "./xhttp", "TestNew_直接调",
       swap('\tif err := cfg.Validate(); err != nil {', '\tif err := cfg.Validate(); false && err != nil {'))
# 「XRedis:」这样的空块曾经让启动直接失败：Has 返回 false，那个包就此跳过、
# 不再 Unmarshal，于是这个 key 没人认领，而 Unclaimed 报出来的两条原因
# （拼错了、忘了 import）都不成立，照着查什么都查不出来
mutate("问过配置就算认领了它", "internal/config/config.go", ".", "TestHas|TestUnclaimed",
       swap('''\tif ensureLocked() != nil {
\t\treturn false
\t}
\tclaimed[key] = true
\tnode, ok := sections[key]''', '''\tif ensureLocked() != nil {
\t\treturn false
\t}
\tnode, ok := sections[key]'''))
# 要防的是：读得早就静默拿到空值，服务带着一套默认配置正常起来。
# 现在第一次读就先加载，这条变异把它改回「没加载就当没配」
mutate("读得早也拿到文件里的值", "internal/config/config.go", ".", "TestUnmarshal_还没加载时先加载|TestRun_在Run之前读配置",
       swap('''\tif err := ensureLocked(); err != nil {
\t\treturn err
\t}
\tclaimed[key] = true''', '''\tif !ready {
\t\treturn nil
\t}
\tclaimed[key] = true'''))
# 从前没有配置文件时配置一直停在「还没加载」，每个集成读配置都报错，
# 一个不需要任何配置的服务根本起不来
mutate("没有配置文件时全用默认值", "internal/config/config.go", ".", "TestEnsure_找不到配置文件|TestRun_找不到配置文件",
       swap('''\t\tsections, claimed, ready = nil, map[string]bool{}, true
\t\treturn nil''', '''\t\tfailed = errors.New("no config file")
\t\treturn failed'''))
mutate("提前加载过的配置不会被悄悄换掉", "internal/config/config.go", ".", "TestEnsure_点名的文件|TestRun_提前读过配置",
       swap('\tif path != "" && !sameFile(path, source) {', '\tif false {'))
# 按字符串比的话 dir/./b.yml、相对路径、符号链接都被当成另一个文件，Run 启动失败
mutate("同一个文件换一种写法也认得出", "internal/config/config.go", ".", "TestEnsure_点名的是同一个文件",
       swap('\tif path != "" && !sameFile(path, source) {', '\tif path != "" && path != source {'))
mutate("经符号链接点名的也是同一个文件", "internal/config/config.go", ".", "TestEnsure_点名的是同一个文件",
       swap('errB == nil && os.SameFile(infoA, infoB)', 'errB == nil && false && os.SameFile(infoA, infoB)'))
mutate("写坏的配置每次读都报同一个错", "internal/config/config.go", ".", "TestUnmarshal_加载失败之后",
       swap('\t\tfailed = err\n\t\treturn err', '\t\treturn err'))
mutate("Run 结束时交还配置", "xone.go", ".", "TestRun_返回之后还能再跑一次", swap('\tdefer config.Reset()\n', ''))
mutate("加载失败的 Run 也交还配置", "xone.go", ".", "TestRun_加载失败之后下一次Run",
       swap('\tdefer config.Reset()\n', ''),
       swap('''\tif err := config.Ensure(o.configPath, o.log()); err != nil {
\t\treturn errors.Join(err, stopWithin(o, nil))
\t}
''', '''\tif err := config.Ensure(o.configPath, o.log()); err != nil {
\t\treturn errors.Join(err, stopWithin(o, nil))
\t}
\tdefer config.Reset()
'''))
# gin 的 ResponseWriter 接口带 WriteString，handler 直接调它是常见写法。
# 包装层只包 Write 的话，这条路写出去的响应在日志里永远是空的
mutate("WriteString 写的响应也截得下来", "xgin/middleware/log.go", "./xgin", "TestLog",
       swap('''\tif w.capture && w.buf.Len() < maxResponseBody {
\t\tw.buf.WriteString(s[:min(len(s), maxResponseBody-w.buf.Len())])
\t}''', '\tif false {\n\t}'))
# gin 的默认是全都信，于是任何人发一个 X-Forwarded-For 就能决定
# 访问日志里的 client_ip。设置出错时必须退到安全的那一侧
mutate("代理网段设不上时退回谁都不信", "xgin/xgin.go", "./xgin", "TestApplyConfig|TestBuild_配了代理",
       swap('\t\t_ = e.SetTrustedProxies([]string{})', ''))

# resty 的默认 logger 绕开 slog 直写 stderr，重试失败时连查询串里的令牌一起打
# XHttp.Trace 只管 Span。原先关掉它连注入一起摘了，透传头和 traceparent 断在这一跳
mutate("XHttp.Trace 关掉照样注入链路标识和透传头", "xhttp/xhttp.go", "./xhttp", "TestNew_关掉Trace",
       swap('next := http.RoundTripper(propagateOnly{next: pool})', 'next := pool'))
mutate("XHttp.Trace 关掉照样按域名透传", "xhttp/xhttp.go", "./xhttp", "TestNew_关掉Trace",
       swap('\treturn &xtrace.Transport{Next: next}\n', '\tif !cfg.Trace {\n\t\treturn next\n\t}\n\treturn &xtrace.Transport{Next: next}\n'))
mutate("只注入那一层不改调用方的请求", "xhttp/xhttp.go", "./xhttp", "TestNew_关掉Trace",
       swap('\tr = r.Clone(r.Context())\n', ''))
mutate("只注入那一层转发 CloseIdleConnections", "xhttp/xhttp.go", "./xhttp", "TestNew_关掉Trace",
       swap('func (p propagateOnly) CloseIdleConnections() {', 'func (p propagateOnly) closeIdleConnections() {'))
# 方法是自由 token，照抄进标签的话谁都能把时间序列撑爆。打在两个调用点上
mutate("出站指标的 method 标签收敛", "xhttp/metric.go", "./xhttp", "TestMetric",
       swap('normalizeMethod(raw.Method)', 'raw.Method', 2))
mutate("出站指标注册失败不让 New 失败", "xhttp/xhttp.go", "./xhttp", "TestNew_指标注册失败",
       swap('\t\t\tslog.Error("xhttp failed to register the request duration metric', '\t\t\treturn nil, nil, err\n\t\t\tslog.Error("xhttp failed to register the request duration metric'))
mutate("resty 自己的日志走 slog", "xhttp/xhttp.go", "./xhttp", "TestNew_resty",
       swap('return resty.NewWithClient(hc).SetLogger(restyLogger{})', 'return resty.NewWithClient(hc)'))
mutate("resty 日志去掉查询串", "xhttp/xhttp.go", "./xhttp", "TestNew_resty",
       swap('"detail", stripQuery(fmt.Sprintf(format, v...))', '"detail", fmt.Sprintf(format, v...)'))
# otelhttp 只去掉 user:password，查询串原样写进 url.full
mutate("出站 Span 的 url.full 不带查询串", "xhttp/xhttp.go", "./xhttp", "TestTransport_Span",
       swap('otelhttp.NewTransport(scrubURL{next: pool},', 'otelhttp.NewTransport(pool,'))
mutate("出站 Span 名只用方法", "xhttp/xhttp.go", "./xhttp", "TestTransport|TestSpanName",
       swap('\treturn r.Method\n}', '\treturn r.Method + " " + r.URL.Path\n}'))
# resty.New() 自带 cookie jar：初始化前、关闭后的请求会共享别人种下的会话
mutate("兜底实例不带 cookie jar", "xhttp/xhttp.go", "./xhttp", "TestC_",
       swap('var fallback = newResty(&http.Client{Timeout: fallbackTimeout})', 'var fallback = resty.New().SetTimeout(fallbackTimeout)'))

section("示例")
# select 在几路同时就绪时随机挑：只靠 select，退出信号到了 worker 还在取消息
mutate("ctx 取消之后不再取消息", "example/consumer/queue.go", "./example", "TestQueue",
       swap('\tif ctx.Err() != nil {\n\t\treturn Message{}, false\n\t}\n\tselect {', '\tselect {'))
# 这两项配错都不报错，服务只是「正常地什么都不做」
mutate("Workers 不大于 0 启动失败", "example/consumer/conf/conf.go", "./example", "TestLoad_",
       swap('if c.Workers <= 0 {', 'if false {'))
mutate("MessageTimeout 不大于 0 启动失败", "example/consumer/conf/conf.go", "./example", "TestLoad_",
       swap('if c.MessageTimeout <= 0 {', 'if false {'))
# 漏登记停止钩子：退出时最后一批改动刷不下去，直接调 closeXKV 的测试照样全绿
mutate("xkv 登记了停止钩子", "example/component/xkv/xkv.go", "./example", "TestRegister_",
       swap('\txhook.BeforeStop(closeXKV, xhook.At(xhook.StageClient))\n', ''))
# FlushInterval 为 0 时 time.NewTicker 在后台协程里 panic，进程直接死掉
mutate("刷盘间隔不大于 0 启动失败", "example/component/xkv/xkv.go", "./example", "TestInitXKV|TestNew_",
       swap('if c.FlushInterval <= 0 {', 'if false {'))
mutate("直接调 New 也校验配置", "example/component/xkv/xkv.go", "./example", "TestNew_",
       swap('\tif err := c.Validate(); err != nil {\n\t\treturn nil, nil, err\n\t}\n', ''))
# 刷盘先清脏标记再写：写失败了不还回去，这批改动连 Close 那次也刷不下去
mutate("刷盘失败不丢数据", "example/component/xkv/xkv.go", "./example", "TestStore",
       swap('\t\ts.mu.Lock()\n\t\ts.dirty = true\n\t\ts.mu.Unlock()\n', ''))


if __name__ == "__main__":
    main()
