"""Hold every macOS deployment target to the floor the Swift manifests declare."""

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]


def macos_floor() -> int:
    """Return the `.macOS(.vN)` major that every Swift manifest declares: the one macOS floor."""
    floors = set()
    for manifest in (
        ROOT / "Package.swift",
        ROOT / "packages" / "swift" / "Package.swift",
        ROOT / "e2e" / "swift_e2e" / "Package.swift",
    ):
        floor = re.findall(r"\.macOS\(\.v(\d+)\)", manifest.read_text())
        assert len(floor) == 1, f"{manifest} must declare exactly one macOS floor"
        floors.add(int(floor[0]))
    assert len(floors) == 1, f"Swift manifests declare different macOS floors: {sorted(floors)}"
    return floors.pop()


def test_every_macos_build_targets_the_manifest_floor() -> None:
    # Every macOS artifact targets the oldest macOS Apple still supports; the Swift manifests
    # carry that floor and every other deployment target must equal it. ~keep
    expected = f"{macos_floor()}.0"
    pins = []
    for workflow in sorted((ROOT / ".github" / "workflows").glob("*.yaml")):
        for value in re.findall(r'MACOSX_DEPLOYMENT_TARGET(?:: "|=)([0-9.]+)', workflow.read_text()):
            pins.append((workflow.name, value))
    pinned_workflows = {name for name, _ in pins}
    assert {"publish.yaml", "ci-python-wheel-smoke.yaml", "qualify-macos-elixir-nif.yaml"} <= pinned_workflows
    assert [pin for pin in pins if pin[1] != expected] == [], f"deployment targets differ from {expected}"

    heif = (ROOT / "scripts" / "ci" / "build-macos-heif-deps.sh").read_text()
    assert f'TARGET="${{MACOSX_DEPLOYMENT_TARGET:-{expected}}}"' in heif
    assert f'[ "$TARGET" != {expected} ]' in heif
    archive = (ROOT / "scripts" / "ci" / "macos_elixir_archive.py").read_text()
    assert f'f"target={expected} arch=arm64' in archive
    assert f'["arm64 {expected}"]' in archive
    alef = re.findall(r'(?m)^min_macos_version = "([0-9.]+)"$', (ROOT / "alef.toml").read_text())
    assert alef == [expected]
    wheel_tags = re.findall(r"macosx_(\d+)_(\d+)_", (ROOT / "cli-proxy" / "pypi" / "hatch_build.py").read_text())
    assert len(wheel_tags) == 2
    assert {f"{major}.{minor}" for major, minor in wheel_tags} == {expected}


if __name__ == "__main__":
    for name, test in list(globals().items()):
        if name.startswith("test_") and callable(test):
            test()
