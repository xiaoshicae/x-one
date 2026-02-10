#!/bin/bash
# 编辑 .rs 文件后，提醒检查对应模块的 README.md 和整体 README.md
# 此脚本由 Claude Code PostToolUse hook 触发

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.pathInProject // ""')

# 只处理 .rs 文件
if [[ "$FILE_PATH" != *.rs ]]; then
  exit 0
fi

# 规范化路径：提取相对于项目根目录的路径
REL_PATH="$FILE_PATH"
if [[ "$REL_PATH" == "$CLAUDE_PROJECT_DIR"* ]]; then
  REL_PATH="${REL_PATH#"$CLAUDE_PROJECT_DIR"/}"
fi

# 提取模块名（src/xorm/client.rs → xorm）
MODULE=""
if [[ "$REL_PATH" =~ ^src/([^/]+)/ ]]; then
  MODULE="${BASH_REMATCH[1]}"
fi

# 无法识别模块时跳过
if [[ -z "$MODULE" ]]; then
  exit 0
fi

# 检查模块 README 是否存在
MODULE_README="$CLAUDE_PROJECT_DIR/src/$MODULE/README.md"
ROOT_README="$CLAUDE_PROJECT_DIR/README.md"

HINTS=""
if [[ -f "$MODULE_README" ]]; then
  HINTS="src/$MODULE/README.md"
fi
if [[ -f "$ROOT_README" ]]; then
  if [[ -n "$HINTS" ]]; then
    HINTS="$HINTS 和 README.md"
  else
    HINTS="README.md"
  fi
fi

if [[ -n "$HINTS" ]]; then
  echo "提示：$MODULE 模块的 .rs 文件已变更，请在任务完成后检查 $HINTS 是否需要同步更新（公开 API、配置项、使用示例等）。"
fi