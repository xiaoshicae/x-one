#!/bin/bash
# 编辑 .rs 文件后自动运行 cargo check，快速反馈编译错误
# 此脚本由 Claude Code PostToolUse hook 触发

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.pathInProject // ""')

if [[ "$FILE_PATH" == *.rs ]]; then
  cd "$CLAUDE_PROJECT_DIR" || exit 0
  OUTPUT=$(cargo check 2>&1)
  EXIT_CODE=$?
  if [ $EXIT_CODE -ne 0 ]; then
    echo "$OUTPUT"
    cat <<EOF
{"decision": "warn", "message": "cargo check 发现编译错误，请修复"}
EOF
  fi
fi
