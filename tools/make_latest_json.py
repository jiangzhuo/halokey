#!/usr/bin/env python3
"""根据已签名的发布产物生成 Tauri updater 清单 latest.json。

updater 会先校验整个文件再比较版本，任何一个平台缺失都会让所有平台无法更新，
所以缺平台或缺签名一律失败。只依赖标准库（Python >= 3.11）。
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO_URL = "https://github.com/jiangzhuo/halokey"

# 平台键 -> 产物文件名正则（键为 updater 的 {os}-{arch}-{installer}）
PLATFORMS = {
    "linux-x86_64-deb": r"_amd64\.deb$",
    "linux-aarch64-deb": r"_arm64\.deb$",
    "darwin-aarch64-app": r"_aarch64\.app\.tar\.gz$",
    "darwin-x86_64-app": r"_x64\.app\.tar\.gz$",
    "windows-x86_64-nsis": r"_x64-setup\.exe$",
    "windows-x86_64-msi": r"_x64_en-US\.msi$",
}


def check_version(requested: str, conf_version: str, cargo_version: str) -> str:
    """去掉前导 v 后，要求请求版本、tauri.conf.json、Cargo.toml 三者一致。"""
    version = requested.removeprefix("v")
    if not (version == conf_version == cargo_version):
        raise ValueError(
            f"版本不一致: 请求 {version!r}, tauri.conf.json {conf_version!r}, "
            f"Cargo.toml {cargo_version!r}"
        )
    return version


def build_manifest(version: str, artifacts: Path) -> dict:
    """为每个平台找到唯一产物及其 .sig；任何缺失或歧义都汇总后抛 ValueError。"""
    names = sorted(p.name for p in artifacts.iterdir() if p.is_file())
    platforms = {}
    problems = []
    for key, pattern in PLATFORMS.items():
        matches = [n for n in names if re.search(pattern, n)]
        if len(matches) != 1:
            problems.append(f"{key}: 需要恰好 1 个匹配 {pattern!r} 的文件, 实际 {matches}")
            continue
        name = matches[0]
        sig = artifacts / f"{name}.sig"
        if not sig.is_file():
            problems.append(f"{key}: 缺少签名文件 {name}.sig")
            continue
        platforms[key] = {
            "url": f"{REPO_URL}/releases/download/v{version}/{name}",
            "signature": sig.read_text(),
        }
    if problems:
        raise ValueError("无法生成 latest.json:\n  " + "\n  ".join(problems))
    return {"version": version, "platforms": platforms}


def read_versions() -> tuple[str, str]:
    conf = json.loads((ROOT / "apps/halokey/tauri.conf.json").read_text())["version"]
    with (ROOT / "Cargo.toml").open("rb") as f:
        cargo = tomllib.load(f)["workspace"]["package"]["version"]
    return conf, cargo


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--artifacts", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    group = ap.add_mutually_exclusive_group(required=True)
    group.add_argument("--tag", help="发布标签, 如 v0.2.0")
    group.add_argument("--version-from-conf", action="store_true")
    args = ap.parse_args()
    try:
        conf, cargo = read_versions()
        requested = conf if args.version_from_conf else args.tag
        version = check_version(requested, conf, cargo)
        manifest = build_manifest(version, args.artifacts)
    except (ValueError, OSError, KeyError) as e:
        print(f"错误: {e}", file=sys.stderr)
        return 1
    args.out.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
