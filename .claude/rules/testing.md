# 测试规则

## 测试位置
- 单元测试：同文件底部 `#[cfg(test)] mod tests { ... }`
- 集成测试：`tests/` 目录，每个文件是独立的测试 crate
- 文档测试：在 `///` 文档注释中的代码块

## 命名规范
- 格式：`test_<被测行为>_<场景>_<预期结果>`
- 示例：`test_add_two_positive_numbers_returns_sum`
- 示例：`test_parse_empty_string_returns_error`

## 断言
- 优先级：`assert_eq!` > `assert_ne!` > `assert!`
- 需要时添加描述消息：`assert_eq!(result, 4, "2 + 2 应该等于 4")`
- 测试 panic 用 `#[should_panic(expected = "消息")]`
- 测试 Result 用 `-> Result<(), Error>` 返回类型

## 覆盖要求
- 每个公开函数至少一个正向测试
- 边界条件和错误路径需要覆盖
- 重构后测试数量不得减少