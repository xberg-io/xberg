"""Verify the local npm release archives against the Node build matrices."""

import json
import re
import tarfile
from pathlib import Path

NODE = Path("crates/xberg-node")
WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/publish.yaml"
MATRICES = ("node-bindings", "node-bindings-musl", "node-bindings-manylinux")


def expected_platforms(workflow: str, main: dict) -> dict[str, tuple[str, str]]:
    targets = []
    for job in MATRICES:
        block = workflow.split(f"\n  {job}:\n", 1)[1].split("\n    steps:", 1)[0]
        targets.extend(re.findall(r"\btarget: ([\w-]+)", block))
    if not targets or sorted(targets) != sorted(main["napi"]["targets"]):
        raise ValueError(f"Node matrices disagree with napi.targets: {targets}")

    platforms = {}
    for target in targets:
        arch, _, system, *abi = target.split("-")
        cpu = {"x86_64": "x64", "aarch64": "arm64"}[arch]
        os_name = {"windows": "win32"}.get(system, system)
        platform = "-".join([os_name, cpu, *abi])
        name = f"{main['name']}-{platform}"
        platforms[name] = (main["version"], f"{main['napi']['binaryName']}.{platform}.node")
    return platforms


def packed_manifest(path: Path) -> tuple[dict, set[str]]:
    with tarfile.open(path, "r:gz") as archive:
        members = {item.name for item in archive.getmembers() if item.isfile() and item.size > 0}
        with archive.extractfile("package/package.json") as manifest:
            return json.load(manifest), members


def verify() -> None:
    source = json.loads((NODE / "package.json").read_text())
    expected = expected_platforms(WORKFLOW.read_text(), source)
    actual = {}
    for path in sorted((NODE / "npm").glob("*.tgz")):
        manifest, members = packed_manifest(path)
        name = manifest["name"]
        if name in actual or name not in expected:
            raise ValueError(f"Duplicate or unexpected platform archive: {name}")
        version, binary = expected[name]
        if manifest["version"] != version or manifest["main"] != binary or f"package/{binary}" not in members:
            raise ValueError(f"Incorrect version or missing native entry in {path}")
        actual[name] = version
    if actual.keys() != expected.keys():
        raise ValueError(f"Missing platform archives: {sorted(expected.keys() - actual.keys())}")

    [path] = Path("dist/node-dry-run").glob("*.tgz")
    manifest, members = packed_manifest(path)
    if (manifest["name"], manifest["version"], manifest.get("optionalDependencies")) != (
        source["name"],
        source["version"],
        actual,
    ):
        raise ValueError("Packed main manifest does not match the verified platform archives")
    required = {f"package/{source['main']}", f"package/{source['types']}", "package/binding.cjs"}
    if missing := required - members:
        raise ValueError(f"Missing main package files: {sorted(missing)}")
    print(f"Verified one main npm archive and {len(actual)} platform archives")


if __name__ == "__main__":
    verify()
