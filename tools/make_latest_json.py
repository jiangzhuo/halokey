#!/usr/bin/env python3
"""根据已签名的发布产物生成 Tauri updater 清单 latest.json。

updater 会先校验整个文件再比较版本，任何一个平台缺失都会让所有平台无法更新，
所以缺平台或缺签名一律失败。签名用的密钥与 tauri.conf.json 里的公钥对不上时，
tauri-cli 只打一行警告，但所有客户端都会拒绝这个更新，所以这里也直接失败。
只依赖标准库（Python >= 3.11）。
"""
import argparse
import base64
import binascii
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO_URL = "https://github.com/jiangzhuo/halokey"
PRODUCT_NAMES = ("HaloKey", "halokey")

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


def _minisign_line2(encoded: str, size: int) -> bytes:
    """Tauri 的公钥和 .sig 都是"minisign 文本再整体 base64 一次"。
    解开外层得到文本，第 2 行又是 base64，解出的二进制必须是 `size` 字节：
    公钥 42 = 算法 2 + 密钥 ID 8 + 公钥 32；签名 74 = 算法 2 + 密钥 ID 8 + 签名 64。
    格式与 minisign-verify 的 `PublicKey::decode` / `Signature::decode` 一致。"""
    try:
        lines = base64.b64decode(encoded.strip(), validate=True).decode("utf-8").splitlines()
        if len(lines) < 2:
            raise ValueError(f"只有 {len(lines)} 行")
        raw = base64.b64decode(lines[1].strip(), validate=True)
    except (binascii.Error, UnicodeDecodeError) as e:
        raise ValueError(f"不是 base64 编码的 minisign 文本 ({e})") from e
    if len(raw) != size:
        raise ValueError(f"第 2 行应为 {size} 字节, 实际 {len(raw)} 字节")
    return raw


def format_key_id(key_id: bytes) -> str:
    """与 minisign 一致：8 字节小端整数的 16 位大写十六进制。"""
    return f"{int.from_bytes(key_id, 'little'):016X}"


def pubkey_key_id(pubkey: str) -> bytes:
    """tauri.conf.json 里 plugins.updater.pubkey 的密钥 ID（第 2 行二进制的第 2..10 字节）。"""
    try:
        raw = _minisign_line2(pubkey, 42)
    except ValueError as e:
        raise ValueError(f"tauri.conf.json 的 plugins.updater.pubkey 无法解析: {e}") from e
    if raw[:2] != b"Ed":
        raise ValueError(f"tauri.conf.json 的公钥算法不支持: {raw[:2]!r}")
    return raw[2:10]


def signature_key_id(signature: str) -> bytes:
    """.sig 内容里签名所用密钥的 ID（第 2 行二进制的第 2..10 字节）。"""
    raw = _minisign_line2(signature, 74)
    if raw[:2] not in (b"Ed", b"ED"):
        raise ValueError(f"签名算法不支持: {raw[:2]!r}")
    return raw[2:10]


def build_manifest(version: str, artifacts: Path, pubkey: str) -> dict:
    """为每个平台找到唯一产物及其 .sig，并确认每个签名都出自 `pubkey` 对应的密钥；
    任何缺失、歧义或密钥不符都汇总后抛 ValueError。"""
    expected = pubkey_key_id(pubkey)
    # 只认文件名以当前版本开头的产物，旧版本残留不会被误用。
    # 文件名前缀跟着 productName：macOS / Windows 是 HaloKey，Linux 的 deb 包名必须小写，是 halokey
    prefixes = tuple(f"{name}_{version}_" for name in PRODUCT_NAMES)
    names = sorted(
        p.name for p in artifacts.iterdir() if p.is_file() and p.name.startswith(prefixes)
    )
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
        signature = sig.read_text()
        try:
            key_id = signature_key_id(signature)
        except ValueError as e:
            problems.append(f"{key}: 签名文件 {name}.sig 无法解析: {e}")
            continue
        if key_id != expected:
            problems.append(
                f"{key}: {name}.sig 的签名密钥 {format_key_id(key_id)} 与 tauri.conf.json "
                f"的公钥 {format_key_id(expected)} 不一致（客户端会拒绝这个更新）"
            )
            continue
        platforms[key] = {
            "url": f"{REPO_URL}/releases/download/v{version}/{name}",
            "signature": signature,
        }
    if problems:
        raise ValueError("无法生成 latest.json:\n  " + "\n  ".join(problems))
    return {"version": version, "platforms": platforms}


def read_tauri_conf() -> dict:
    return json.loads((ROOT / "apps/halokey/tauri.conf.json").read_text())


def read_versions() -> tuple[str, str]:
    conf = read_tauri_conf()["version"]
    with (ROOT / "Cargo.toml").open("rb") as f:
        cargo = tomllib.load(f)["workspace"]["package"]["version"]
    return conf, cargo


def read_pubkey() -> str:
    """应用内置的更新公钥（客户端用它验签）。"""
    return read_tauri_conf()["plugins"]["updater"]["pubkey"]


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
        manifest = build_manifest(version, args.artifacts, read_pubkey())
    except (ValueError, OSError, KeyError) as e:
        print(f"错误: {e}", file=sys.stderr)
        return 1
    args.out.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
