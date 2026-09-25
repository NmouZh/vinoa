#!/usr/bin/env bash
# 端到端验收 harness（spec §12.2）。本脚本由 Lead 维护，是"生成物真能构建"的判据。
#
# 用法:
#   scripts/acceptance.sh              # 默认 A4 行：1.21.11 + paper(+bukkit)
#   scripts/acceptance.sh 1.8.9 bukkit # A1 行
#   scripts/acceptance.sh 26.2 paper   # A5 行
set -u

MC="${1:-1.21.11}"
PLATFORM="${2:-paper}"
NAME="my-plugin"
PKG="com.example.myplugin"

echo "== 构建 vinoa =="
cargo build --quiet || { echo "FAIL: cargo build"; exit 1; }
BIN="$(pwd)/target/debug/vinoa"

WORK=$(mktemp -d)
cd "$WORK" || exit 1
echo "工作目录: $WORK"
echo

echo "== 1) --dry-run --json（计划必须可解析、且只输出一个 JSON） =="
"$BIN" init "$NAME" -p "$PKG" -m "$MC" --platform "$PLATFORM" -y --dry-run --json > plan.json
if python3 -c "import json,sys; json.load(open('plan.json'))" 2>/dev/null; then
  echo "  ✓ plan.json 是合法 JSON"
  python3 - <<'PY'
import json
d = json.load(open("plan.json"))
files = d.get("files") or (d.get("plan") or {}).get("files") or []
print(f"  ✓ 计划包含 {len(files)} 个文件")
PY
else
  echo "  ✗ plan.json 不是合法 JSON"; head -c 300 plan.json; exit 1
fi
echo

echo "== 2) 真实生成 =="
"$BIN" init "$NAME" -p "$PKG" -m "$MC" --platform "$PLATFORM" -y --no-git || { echo "FAIL: init"; exit 1; }
test -d "$NAME" || { echo "FAIL: 目标目录不存在"; exit 1; }
echo "  生成文件数: $(find "$NAME" -type f | wc -l)"
echo "  渲染残留自查（应无输出）:"
grep -rIn --exclude-dir=.git -e 'com\.example' -e '{{' -e '}}' "$NAME" | head -10 || true
echo

echo "== 3) ./gradlew build（这是验收的核心判据） =="
cd "$NAME" || exit 1
if [ -x ./gradlew ]; then
  timeout 1800 ./gradlew build --no-daemon 2>&1 | tail -25
  rc=${PIPESTATUS[0]}
else
  timeout 1800 gradle build --no-daemon 2>&1 | tail -25
  rc=${PIPESTATUS[0]}
fi
if [ "$rc" -ne 0 ]; then
  echo "FAIL: gradlew build 退出码 $rc"
  exit 1
fi

echo
echo "== 4) 产物检查 =="
JARS=$(find . -path ./.gradle -prune -o -name '*.jar' -path '*build/libs*' -print)
echo "$JARS"
if [ -z "$JARS" ]; then
  echo "FAIL: build/libs 下没有 jar"
  exit 1
fi
echo
echo "✓ 验收通过：MC=$MC platform=$PLATFORM（工程在 $WORK/$NAME）"
