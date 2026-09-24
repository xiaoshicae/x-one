package main

import (
	"fmt"
	"maps"
	"regexp"
	"slices"
	"strings"

	"go.yaml.in/yaml/v3"
)

// checkDocs 对照 schema 检查 docs/config.md，返回全部问题。
//
// 文档的结构是固定的：每个配置块一节，标题以它的顶层 key 开头
// （`## XLog —— 日志`），这一节里有它的 YAML 示例。两个方向都查，而且都按节查：
//
//   - 每个字段都要在**它自己那一节**里出现。从前的检查是在整份文档里 grep
//     字段名，Name、Timeout、Enable 这种名字总能在别的节里找到，永远通过。
//   - 每一节里的 YAML 示例都得是真的能用的配置：字段存在、类型对、
//     单实例和多实例的写法没混。文档里多写一个不存在的 key，
//     照着配的人会发现它不生效——运行时直接启动失败。
func checkDocs(r *root, md string) []string {
	rs, err := compile(r)
	if err != nil {
		return []string{fmt.Sprintf("schema 本身不合法：%v", err)}
	}
	sections := docSections(md)
	var out []string
	for _, key := range slices.Sorted(maps.Keys(r.Properties)) {
		text, ok := sections[key]
		if !ok {
			out = append(out, fmt.Sprintf("没有「## %s」这一节", key))
			continue
		}
		for _, f := range fieldNames(r.Properties[key]) {
			if !regexp.MustCompile(`\b` + regexp.QuoteMeta(f) + `\b`).MatchString(text) {
				out = append(out, fmt.Sprintf("「## %s」一节里没有提到字段 %s", key, f))
			}
		}
		for i, block := range yamlBlocks(text) {
			var doc map[string]any
			if err := yaml.Unmarshal([]byte(block), &doc); err != nil {
				out = append(out, fmt.Sprintf("「## %s」第 %d 个 YAML 示例解析失败：%v", key, i+1, err))
				continue
			}
			for _, top := range slices.Sorted(maps.Keys(doc)) {
				if _, ok := r.Properties[top]; !ok {
					out = append(out, fmt.Sprintf("「## %s」第 %d 个 YAML 示例里有不存在的顶层 key %s", key, i+1, top))
				}
			}
			for _, p := range problems(rs, doc) {
				out = append(out, fmt.Sprintf("「## %s」第 %d 个 YAML 示例：%s", key, i+1, p))
			}
		}
	}
	return out
}

// docSections 按二级标题切开文档：标题的第一个词 → 这一节的全文（含三级标题下的内容）
func docSections(md string) map[string]string {
	out := map[string]string{}
	var key string
	var body strings.Builder
	flush := func() {
		if key != "" {
			out[key] = body.String()
		}
		body.Reset()
	}
	for _, line := range strings.SplitAfter(md, "\n") {
		if title, ok := strings.CutPrefix(line, "## "); ok {
			flush()
			key = strings.Fields(title + " ")[0]
			continue
		}
		body.WriteString(line)
	}
	flush()
	return out
}

// yamlBlocks 取出一段 Markdown 里所有 ```yaml 代码块的内容
func yamlBlocks(text string) []string {
	var out []string
	var cur *strings.Builder
	for _, line := range strings.SplitAfter(text, "\n") {
		trimmed := strings.TrimSpace(line)
		switch {
		case cur == nil && trimmed == "```yaml":
			cur = &strings.Builder{}
		case cur != nil && trimmed == "```":
			out = append(out, cur.String())
			cur = nil
		case cur != nil:
			cur.WriteString(line)
		}
	}
	return out
}

// fieldNames 一个配置块里出现的全部字段名，去重排序。
// map 的 key 是使用者自己起的名字（实例名、标签名），不算字段
func fieldNames(n *node) []string {
	seen := map[string]bool{}
	var walk func(*node)
	walk = func(n *node) {
		if n == nil {
			return
		}
		for name, p := range n.Properties {
			seen[name] = true
			walk(p)
		}
		walk(n.Items)
		if ap, ok := n.AdditionalProperties.(*node); ok {
			walk(ap)
		}
		for _, b := range n.OneOf {
			walk(b)
		}
	}
	walk(n)
	return slices.Sorted(maps.Keys(seen))
}
