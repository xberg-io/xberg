"""The Ruby vendor manifests retain the source workspace's Rust requirements."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import tomllib


class VendorRustVersionTests(unittest.TestCase):
    def vendor(self, workspace_field: str) -> tuple[Path, subprocess.CompletedProcess[str]]:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / "Cargo.toml").write_text(
            f'[workspace.package]\nversion = "1.4.0"\n{workspace_field}\n', encoding="utf-8"
        )
        for name, requirement in [("xberg", "rust-version.workspace = true"), ("xberg-ffi", 'rust-version = "1.96"')]:
            source = root / "crates" / name
            source.mkdir(parents=True)
            (source / "Cargo.toml").write_text(
                f'[package]\nname = "{name}"\nversion.workspace = true\nedition.workspace = true\n{requirement}\n',
                encoding="utf-8",
            )
        result = subprocess.run(
            [sys.executable, str(Path(__file__).with_name("vendor-xberg-core.py"))],
            env={**os.environ, "REPO_ROOT": str(root)},
            capture_output=True,
            text=True,
            check=False,
        )
        return root / "packages/ruby/vendor", result

    def test_generated_workspace_follows_source_rust_version(self) -> None:
        for version in ["1.98", "1.99.1"]:
            with self.subTest(version=version):
                vendor, result = self.vendor(f'rust-version = "{version}"')
                assert result.returncode == 0, result.stderr
                workspace = tomllib.loads((vendor / "Cargo.toml").read_text())
                assert workspace["workspace"]["members"] == ["xberg", "xberg-ffi"]
                assert workspace["workspace"]["package"]["rust-version"] == version

    def test_inherited_package_follows_source_rust_version(self) -> None:
        for version in ["1.98", "1.99.1"]:
            with self.subTest(version=version):
                vendor, result = self.vendor(f'rust-version = "{version}"')
                assert result.returncode == 0, result.stderr
                package = tomllib.loads((vendor / "xberg/Cargo.toml").read_text())
                assert package["package"]["rust-version"] == version

    def test_explicit_package_override_remains_unchanged(self) -> None:
        vendor, result = self.vendor('rust-version = "1.98"')
        assert result.returncode == 0, result.stderr
        package = tomllib.loads((vendor / "xberg-ffi/Cargo.toml").read_text())
        assert package["package"]["rust-version"] == "1.96"

    def test_missing_workspace_rust_version_fails_before_vendoring(self) -> None:
        vendor, result = self.vendor("")
        assert result.returncode != 0
        assert "workspace.package.rust-version" in result.stderr
        assert not vendor.exists()

    def test_invalid_workspace_rust_versions_fail_before_vendoring(self) -> None:
        for value in ['""', '"nightly"', '"^1.98"', "1.98", "true"]:
            with self.subTest(value=value):
                vendor, result = self.vendor(f"rust-version = {value}")
                assert result.returncode != 0
                assert "workspace.package.rust-version" in result.stderr
                assert not vendor.exists()


if __name__ == "__main__":
    unittest.main()
