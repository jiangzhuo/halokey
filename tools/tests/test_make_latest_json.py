import base64
import sys
import tempfile
import unittest
from pathlib import Path

# tools/ 不是包，把它放进 sys.path 才能 import make_latest_json
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from make_latest_json import (  # noqa: E402
    PLATFORMS,
    build_manifest,
    check_version,
    format_key_id,
    pubkey_key_id,
    read_pubkey,
    signature_key_id,
)

# 测试用的密钥 ID（8 字节，任意取值；不涉及真实密码学）
KEY_ID = bytes.fromhex("0123456789abcdef")
OTHER_KEY_ID = bytes.fromhex("fedcba9876543210")


def b64(data) -> str:
    if isinstance(data, str):
        data = data.encode()
    return base64.b64encode(data).decode()


def fake_pubkey(key_id=KEY_ID) -> str:
    """与 tauri.conf.json 同格式的公钥：minisign 公钥文本再整体 base64；第 2 行是 "Ed" + 密钥 ID + 32 字节公钥。"""
    text = f"untrusted comment: minisign public key: {format_key_id(key_id)}\n{b64(b'Ed' + key_id + bytes(32))}\n"
    return b64(text)


def fake_signature(name, key_id=KEY_ID) -> str:
    """与 tauri-cli 写出的 .sig 同格式：minisign 签名文本再整体 base64；第 2 行是 "ED" + 密钥 ID + 64 字节签名。"""
    text = (
        "untrusted comment: signature from tauri secret key\n"
        f"{b64(b'ED' + key_id + bytes(64))}\n"
        f"trusted comment: timestamp:1791649051\tfile:{name}\n"
        f"{b64(bytes(64))}"
    )
    return b64(text)


PUBKEY = fake_pubkey()


def package_names(version):
    return [
        f"halokey_{version}_amd64.deb",
        f"halokey_{version}_arm64.deb",
        f"HaloKey_{version}_aarch64.app.tar.gz",
        f"HaloKey_{version}_x64.app.tar.gz",
        f"HaloKey_{version}_x64-setup.exe",
        f"HaloKey_{version}_x64_en-US.msi",
    ]


def fake_artifacts(test, version, skip=None, skip_sig=None):
    """在临时目录里造六个包和同名 .sig（用 KEY_ID 签的 minisign 格式签名），外加应被忽略的 .dmg。"""
    tmp = tempfile.TemporaryDirectory()
    test.addCleanup(tmp.cleanup)
    d = Path(tmp.name)
    for name in package_names(version):
        if name == skip:
            continue
        (d / name).write_bytes(b"binary")
        if name != skip_sig:
            (d / f"{name}.sig").write_text(fake_signature(name))
    (d / f"HaloKey_{version}_aarch64.dmg").write_bytes(b"dmg")
    return d


class MakeLatestJson(unittest.TestCase):
    def test_full_manifest(self):
        m = build_manifest("0.2.0", fake_artifacts(self, "0.2.0"), PUBKEY)
        self.assertEqual(m["version"], "0.2.0")
        self.assertNotIn("notes", m)
        self.assertEqual(set(m["platforms"]), set(PLATFORMS))
        self.assertNotIn("linux-x86_64", m["platforms"])
        deb = m["platforms"]["linux-aarch64-deb"]
        self.assertEqual(
            deb["url"],
            "https://github.com/jiangzhuo/halokey/releases/download/v0.2.0/halokey_0.2.0_arm64.deb",
        )
        self.assertEqual(deb["signature"], fake_signature("halokey_0.2.0_arm64.deb"))
        # macOS / Windows 的文件名前缀是 HaloKey，Linux deb 是小写 halokey，两种都要认
        self.assertTrue(m["platforms"]["darwin-aarch64-app"]["url"].endswith("/HaloKey_0.2.0_aarch64.app.tar.gz"))

    def test_unknown_prefix_ignored(self):
        d = fake_artifacts(self, "0.2.0", skip="HaloKey_0.2.0_x64-setup.exe")
        (d / "Other_0.2.0_x64-setup.exe").write_bytes(b"x")
        (d / "Other_0.2.0_x64-setup.exe.sig").write_text(fake_signature("Other_0.2.0_x64-setup.exe"))
        with self.assertRaisesRegex(ValueError, "windows-x86_64-nsis"):
            build_manifest("0.2.0", d, PUBKEY)

    def test_missing_platform_fails(self):
        d = fake_artifacts(self, "0.2.0", skip="halokey_0.2.0_arm64.deb")
        with self.assertRaisesRegex(ValueError, "linux-aarch64-deb"):
            build_manifest("0.2.0", d, PUBKEY)

    def test_missing_signature_fails(self):
        d = fake_artifacts(self, "0.2.0", skip_sig="HaloKey_0.2.0_x64-setup.exe")
        with self.assertRaisesRegex(ValueError, "windows-x86_64-nsis"):
            build_manifest("0.2.0", d, PUBKEY)

    def test_ambiguous_platform_fails(self):
        d = fake_artifacts(self, "0.2.0")
        # 同版本的第二个文件也匹配 amd64.deb 的正则
        (d / "halokey_0.2.0_extra_amd64.deb").write_bytes(b"x")
        (d / "halokey_0.2.0_extra_amd64.deb.sig").write_text("s")
        with self.assertRaisesRegex(ValueError, "linux-x86_64-deb"):
            build_manifest("0.2.0", d, PUBKEY)

    def test_stale_version_artifacts_ignored(self):
        # 目录里只有旧版本产物: 不能拿来生成新版本清单
        d = fake_artifacts(self, "0.1.9")
        with self.assertRaises(ValueError) as cm:
            build_manifest("0.2.0", d, PUBKEY)
        for key in PLATFORMS:
            self.assertIn(key, str(cm.exception))

    def test_stale_files_do_not_shadow_current(self):
        d = fake_artifacts(self, "0.2.0")
        (d / "halokey_0.1.9_amd64.deb").write_bytes(b"x")
        (d / "halokey_0.1.9_amd64.deb.sig").write_text("s")
        m = build_manifest("0.2.0", d, PUBKEY)
        self.assertIn("0.2.0", m["platforms"]["linux-x86_64-deb"]["url"])

    def test_signature_from_another_key_fails_naming_the_file(self):
        d = fake_artifacts(self, "0.2.0")
        name = "HaloKey_0.2.0_x64-setup.exe"
        (d / f"{name}.sig").write_text(fake_signature(name, OTHER_KEY_ID))
        with self.assertRaises(ValueError) as cm:
            build_manifest("0.2.0", d, PUBKEY)
        msg = str(cm.exception)
        self.assertIn(f"{name}.sig", msg)
        self.assertIn("windows-x86_64-nsis", msg)
        self.assertIn(format_key_id(OTHER_KEY_ID), msg)
        self.assertIn(format_key_id(KEY_ID), msg)
        # 其他平台的签名没问题，不应出现在错误里
        self.assertNotIn("linux-x86_64-deb", msg)

    def test_pubkey_from_another_key_fails_every_platform(self):
        d = fake_artifacts(self, "0.2.0")
        with self.assertRaises(ValueError) as cm:
            build_manifest("0.2.0", d, fake_pubkey(OTHER_KEY_ID))
        for key in PLATFORMS:
            self.assertIn(key, str(cm.exception))

    def test_unparsable_signature_fails_naming_the_file(self):
        d = fake_artifacts(self, "0.2.0")
        (d / "halokey_0.2.0_amd64.deb.sig").write_text("SIG-not-minisign")
        with self.assertRaisesRegex(ValueError, r"halokey_0\.2\.0_amd64\.deb\.sig"):
            build_manifest("0.2.0", d, PUBKEY)

    def test_unparsable_pubkey_fails(self):
        d = fake_artifacts(self, "0.2.0")
        with self.assertRaisesRegex(ValueError, "pubkey"):
            build_manifest("0.2.0", d, "not-a-key")

    def test_key_id_layout_matches_minisign(self):
        # 密钥 ID 取第 2 行二进制的第 2..10 字节，显示为小端整数的十六进制（与公钥注释里的一致）
        self.assertEqual(pubkey_key_id(PUBKEY), KEY_ID)
        self.assertEqual(format_key_id(KEY_ID), "EFCDAB8967452301")

    def test_real_tauri_signature_vector(self):
        # tauri-plugin-updater 2.14.0 src/updater.rs 测试里的公开向量（真实的 tauri-cli 签名），
        # 按 Tauri 的方式整体 base64 后解析：签名与公钥的密钥 ID 一致，也与注释里的一致
        pubkey = b64(
            "untrusted comment: minisign public key: A49EF4746FA6E6D7\n"
            "RWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe"
        )
        signature = b64(
            "untrusted comment: signature from tauri secret key\n"
            "RUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\n"
            "trusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\n"
            "DDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA=="
        )
        self.assertEqual(format_key_id(pubkey_key_id(pubkey)), "A49EF4746FA6E6D7")
        self.assertEqual(signature_key_id(signature), pubkey_key_id(pubkey))

    def test_real_config_pubkey_parses(self):
        # 仓库里 tauri.conf.json 的公钥能解析，且与它自己的注释写的 ID 一致
        pubkey = read_pubkey()
        comment = base64.b64decode(pubkey).decode().splitlines()[0]
        self.assertTrue(comment.endswith(format_key_id(pubkey_key_id(pubkey))), comment)

    def test_version_must_match_everywhere(self):
        self.assertEqual(check_version("v0.2.0", "0.2.0", "0.2.0"), "0.2.0")
        with self.assertRaises(ValueError):
            check_version("v0.2.1", "0.2.0", "0.2.0")
        with self.assertRaises(ValueError):
            check_version("v0.2.0", "0.2.0", "0.2.1")


if __name__ == "__main__":
    unittest.main()
