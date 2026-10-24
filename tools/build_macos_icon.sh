#!/usr/bin/env bash
# 把 apps/halokey/icons/AppIcon.icon（Icon Composer 格式）编译成 apps/halokey/icons/Assets.car。
# macOS 26 只认 Assets.car 里的原生图标；只有 icns 时，系统会把图标缩小后放在灰色底板上显示。
# 需要 Xcode 26 或更新版本的 actool。产物提交进仓库，CI 不用再编译。
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
icons="$root/apps/halokey/icons"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# actool 的诊断写在 stdout：留到日志里，失败时打印出来。
if ! xcrun actool "$icons/AppIcon.icon" \
  --compile "$tmp" \
  --app-icon AppIcon \
  --platform macosx \
  --minimum-deployment-target 10.13 \
  --output-partial-info-plist "$tmp/partial.plist" >"$tmp/actool.log" 2>&1; then
  echo "actool 失败：" >&2
  cat "$tmp/actool.log" >&2
  exit 1
fi
# Xcode 26 之前的 actool 不认 .icon，可能"成功"退出却不生成 Assets.car。
if [[ ! -f "$tmp/Assets.car" ]]; then
  echo "actool 没有生成 Assets.car（需要 Xcode 26 或更新版本）。actool 输出：" >&2
  cat "$tmp/actool.log" >&2
  exit 1
fi
cp "$tmp/Assets.car" "$icons/Assets.car"
echo "已生成 $icons/Assets.car"
