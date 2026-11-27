"""Tests for the environment the proxy hands the native binary (ADR 0022)."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest
from xberg_cli import cli, downloader

_VAR = "MallocMaxMagazines"


def _use_system(monkeypatch, name: str) -> None:
    monkeypatch.setattr(downloader.platform, "system", lambda: name)


def test_should_set_one_magazine_on_macos_when_the_user_has_not(monkeypatch) -> None:
    _use_system(monkeypatch, "Darwin")
    monkeypatch.delenv(_VAR, raising=False)
    monkeypatch.setenv("XBERG_PROXY_TEST_MARKER", "kept")

    env = downloader.native_env()

    assert env is not None
    assert env[_VAR] == "1"
    assert env["XBERG_PROXY_TEST_MARKER"] == "kept"


@pytest.mark.parametrize("value", ["4", ""])
def test_should_keep_a_value_the_user_set_on_macos(monkeypatch, value: str) -> None:
    _use_system(monkeypatch, "Darwin")
    monkeypatch.setenv(_VAR, value)

    assert downloader.native_env() is None


@pytest.mark.parametrize("system", ["Linux", "Windows"])
def test_should_leave_the_environment_alone_off_macos(monkeypatch, system: str) -> None:
    _use_system(monkeypatch, system)
    monkeypatch.delenv(_VAR, raising=False)

    assert downloader.native_env() is None


@pytest.mark.skipif(sys.platform == "win32", reason="the stand-in binary is a POSIX script")
@pytest.mark.parametrize(
    ("system", "preset", "expected"),
    [("Darwin", None, "1"), ("Darwin", "4", "4"), ("Linux", None, "<unset>")],
)
def test_should_pass_the_magazine_setting_to_the_bundled_binary(
    tmp_path: Path, monkeypatch, system: str, preset: str | None, expected: str
) -> None:
    report = tmp_path / "report"
    binary = tmp_path / "xberg"
    binary.write_text(f'#!/bin/sh\nprintf "%s" "${{{_VAR}-<unset>}}" > "{report}"\n')
    binary.chmod(0o755)
    _use_system(monkeypatch, system)
    if preset is None:
        monkeypatch.delenv(_VAR, raising=False)
    else:
        monkeypatch.setenv(_VAR, preset)
    monkeypatch.setattr(cli, "_find_bundled_binary", lambda: str(binary))
    monkeypatch.setattr(sys, "argv", ["xberg", "--version"])

    with pytest.raises(SystemExit) as exited:
        cli.main()

    assert exited.value.code == 0
    assert report.read_text() == expected
