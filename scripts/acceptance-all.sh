#!/usr/bin/env bash
# 跑完验收矩阵的其余行（A2/A3/A6/A7 + B 级渲染）。Lead 的验收入口之一。
set -u
cd "$(dirname "$0")/.." || exit 1
cargo build --quiet || exit 1
BIN="$(pwd)/target/debug/vinoa"

run_row() { # mc platform expect_build
  local mc="$1" platform="$2" expect="$3"
  local tmp; tmp=$(mktemp -d)
  local ok="FAIL"
  (
    cd "$tmp" || exit 1
    "$BIN" init my-plugin -p com.example.myplugin -m "$mc" --platform "$platform" -y --no-git >/dev/null 2>&1 || exit 20
    cd my-plugin || exit 21
    if [ "$expect" = "build" ]; then
      timeout 1800 ./gradlew build --no-daemon >build.log 2>&1 || exit 22
    else
      timeout 1200 ./gradlew assemble --no-daemon >build.log 2>&1 || exit 23
    fi
  )
  local rc=$?
  if [ $rc -eq 0 ]; then ok="PASS"; fi
  printf '%-6s %-9s %-12s rc=%s\n' "$ok" "$mc" "$platform" "$rc"
  if [ $rc -ne 0 ]; then tail -12 "$tmp/my-plugin/build.log" 2>/dev/null; fi
  rm -rf "$tmp"
}

echo "=== A 级剩余行（真跑 build） ==="
run_row 1.12.2 paper build
run_row 1.16.5 paper build
run_row 1.21.11 velocity build
run_row 1.21.11 bungeecord build

echo
echo "=== B 级（渲染 + 依赖解析） ==="
run_row 26.2 folia resolve
run_row 26.2 sponge resolve
run_row 26.2 minestom resolve

echo
echo "=== 完成 ==="
