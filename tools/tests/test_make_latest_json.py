import sys
import tempfile
import unittest
from pathlib import Path

# tools/ 不是包，把它放进 sys.path 才能 import make_latest_json
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from make_latest_json import PLATFORMS, build_manifest, check_version  # noqa: E402


def package_names(version):
    return [
        f"halokey_{version}_amd64.deb",
        f"halokey_{version}_arm64.deb",
        f"halokey_{version}_aarch64.app.tar.gz",
        f"halokey_{version}_x64.app.tar.gz",
        f"halokey_{version}_x64-setup.exe",
        f"halokey_{version}_x64_en-US.msi",
    ]


def fake_artifacts(test, version, skip=None, skip_sig=None):
    """在临时目录里造六个包和同名 .sig（签名内容为 SIG-<文件名>），外加应被忽略的 .dmg。"""
    tmp = tempfile.TemporaryDirectory()
    test.addCleanup(tmp.cleanup)
    d = Path(tmp.name)
    for name in package_names(version):
        if name == skip:
            continue
        (d / name).write_bytes(b"binary")
        if name != skip_sig:
            (d / f"{name}.sig").write_text(f"SIG-{name}")
    (d / f"halokey_{version}_aarch64.dmg").write_bytes(b"dmg")
    return d


class MakeLatestJson(unittest.TestCase):
    def test_full_manifest(self):
        m = build_manifest("0.2.0", fake_artifacts(self, "0.2.0"))
        self.assertEqual(m["version"], "0.2.0")
        self.assertNotIn("notes", m)
        self.assertEqual(set(m["platforms"]), set(PLATFORMS))
        self.assertNotIn("linux-x86_64", m["platforms"])
        deb = m["platforms"]["linux-aarch64-deb"]
        self.assertEqual(
            deb["url"],
            "https://github.com/jiangzhuo/halokey/releases/download/v0.2.0/halokey_0.2.0_arm64.deb",
        )
        self.assertEqual(deb["signature"], "SIG-halokey_0.2.0_arm64.deb")

    def test_missing_platform_fails(self):
        d = fake_artifacts(self, "0.2.0", skip="halokey_0.2.0_arm64.deb")
        with self.assertRaisesRegex(ValueError, "linux-aarch64-deb"):
            build_manifest("0.2.0", d)

    def test_missing_signature_fails(self):
        d = fake_artifacts(self, "0.2.0", skip_sig="halokey_0.2.0_x64-setup.exe")
        with self.assertRaisesRegex(ValueError, "windows-x86_64-nsis"):
            build_manifest("0.2.0", d)

    def test_ambiguous_platform_fails(self):
        d = fake_artifacts(self, "0.2.0")
        # 同版本的第二个文件也匹配 amd64.deb 的正则
        (d / "halokey_0.2.0_extra_amd64.deb").write_bytes(b"x")
        (d / "halokey_0.2.0_extra_amd64.deb.sig").write_text("s")
        with self.assertRaisesRegex(ValueError, "linux-x86_64-deb"):
            build_manifest("0.2.0", d)

    def test_stale_version_artifacts_ignored(self):
        # 目录里只有旧版本产物: 不能拿来生成新版本清单
        d = fake_artifacts(self, "0.1.9")
        with self.assertRaises(ValueError) as cm:
            build_manifest("0.2.0", d)
        for key in PLATFORMS:
            self.assertIn(key, str(cm.exception))

    def test_stale_files_do_not_shadow_current(self):
        d = fake_artifacts(self, "0.2.0")
        (d / "halokey_0.1.9_amd64.deb").write_bytes(b"x")
        (d / "halokey_0.1.9_amd64.deb.sig").write_text("s")
        m = build_manifest("0.2.0", d)
        self.assertIn("0.2.0", m["platforms"]["linux-x86_64-deb"]["url"])

    def test_version_must_match_everywhere(self):
        self.assertEqual(check_version("v0.2.0", "0.2.0", "0.2.0"), "0.2.0")
        with self.assertRaises(ValueError):
            check_version("v0.2.1", "0.2.0", "0.2.0")
        with self.assertRaises(ValueError):
            check_version("v0.2.0", "0.2.0", "0.2.1")


if __name__ == "__main__":
    unittest.main()
