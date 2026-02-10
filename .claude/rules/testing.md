# 测试规则

## 测试位置
- **集成测试优先**：所有针对公开 API（`pub fn`/`pub struct` 方法）的测试放在 `tests/` 目录
- **单元测试仅用于私有逻辑**：只有无法通过公开 API 覆盖的私有函数/内部逻辑，才在同文件底部用 `#[cfg(test)] mod tests { ... }`
- 文档测试：在 `///` 文档注释中的代码块
- **判断原则**：如果一个测试只使用 `pub` 接口，它就应该在 `tests/` 中；如果必须访问 `pub(crate)` 或私有成员，才放在 `src/` 的 `#[cfg(test)]` 中

## 测试目录结构
```
tests/
├── xmodule.rs           # 顶层入口，用 #[path] 导入子模块
└── xmodule/
    ├── mod.rs           # 子模块声明
    ├── config.rs        # 配置结构体测试
    ├── client.rs        # API 测试
    └── init.rs          # 初始化集成测试
```

## 命名规范
- 格式：`test_<被测行为>_<场景>_<预期结果>`
- 示例：`test_add_two_positive_numbers_returns_sum`
- 示例：`test_parse_empty_string_returns_error`

## 断言
- 优先级：`assert_eq!` > `assert_ne!` > `assert!`
- 需要时添加描述消息：`assert_eq!(result, 4, "2 + 2 应该等于 4")`
- 测试 panic 用 `#[should_panic(expected = "消息")]`
- 测试 Result 用 `-> Result<(), Error>` 返回类型

## 全局状态测试
- 修改全局状态的测试必须标注 `#[serial]`（`serial_test` crate）
- 每个测试开头调用 `reset_xxx()` 清理上一个测试的残留
- 需要 Tokio runtime 的测试用 `#[tokio::test]`（sqlx `connect_lazy` 等）

## 覆盖要求
- 每个公开函数至少一个正向测试
- 边界条件和错误路径需要覆盖
- 重构后测试数量不得减少