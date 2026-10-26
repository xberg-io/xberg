"""Verify Pi and OpenCode resources inside the actual npm release archive."""

import json
import subprocess
import tarfile
import tempfile
from pathlib import Path


def verify_archive(archive: Path, expected_name: str, expected_skills: int, expected_prompts: int) -> None:
    with tarfile.open(archive) as packed:
        files = set(packed.getnames())
        metadata = json.load(packed.extractfile("package/package.json"))
    if metadata["name"] != expected_name:
        raise ValueError("Plugin npm package name changed")
    if "pi-package" not in metadata.get("keywords", []):
        raise ValueError("Pi discovery keyword is missing")
    expected_resources = {
        "skills": sorted(
            "./" + name.removeprefix("package/").removesuffix("/SKILL.md")
            for name in files
            if name.startswith("package/.pi/skills/") and name.endswith("/SKILL.md")
        ),
        "prompts": sorted(
            "./" + name.removeprefix("package/")
            for name in files
            if name.startswith("package/.pi/prompts/") and name.endswith(".md")
        ),
    }
    actual_resources = metadata.get("pi", {})
    if set(actual_resources) != set(expected_resources) or any(
        sorted(actual_resources[kind]) != paths for kind, paths in expected_resources.items()
    ):
        raise ValueError("Pi resource paths must name only the packed product skills and prompts")
    required = {
        "package/" + metadata["main"],
        "package/.opencode/ai-rulez-content.js",
        "package/.opencode/ai-rulez-bundle.json",
        "package/scripts/mcp-launch.sh",
    }
    if not required.issubset(files):
        raise ValueError("OpenCode entrypoint, bundled content, or MCP launcher is missing")
    for prefix, expected in (("skills", expected_skills), ("prompts", expected_prompts)):
        actual = sum(
            name.startswith(f"package/.pi/{prefix}/")
            and (name.endswith("/SKILL.md") if prefix == "skills" else name.endswith(".md"))
            for name in files
        )
        if expected == 0 or actual != expected:
            raise ValueError(f"Pi {prefix}: expected {expected} files, packed {actual}")
        print(f"Pi {prefix}: expected {expected}, packed {actual}")


def verify_root_pi(root: Path) -> None:
    """Require the root package.json Pi lists to resolve and to name the plugin's resources."""
    root_pi = json.loads((root / "package.json").read_text()).get("pi", {})
    plugin_pi = json.loads((root / "plugin/package.json").read_text()).get("pi", {})
    for kind in ("skills", "prompts"):
        for entry in root_pi.get(kind, []):
            path = root / entry
            found = (path / "SKILL.md").is_file() if kind == "skills" else path.is_file()
            if not found:
                raise ValueError(f"Root package.json pi.{kind} path does not resolve: {entry}")
        listed = [Path(entry).name for entry in root_pi.get(kind, [])]
        duplicates = sorted({name for name in listed if listed.count(name) > 1})
        if duplicates:
            raise ValueError(f"Root package.json pi.{kind} lists a name twice: {duplicates}")
        root_names = set(listed)
        plugin_names = {Path(entry).name for entry in plugin_pi.get(kind, [])}
        if root_names != plugin_names:
            raise ValueError(
                f"Root package.json pi.{kind} differs from plugin/package.json: {sorted(root_names ^ plugin_names)}"
            )


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    verify_root_pi(root)
    plugin = root / "plugin"
    metadata = json.loads((plugin / "package.json").read_text())
    skills = len(list((plugin / ".ai-rulez/skills").glob("*/SKILL.md")))
    prompts = sum(path.name not in {"index.md", "log.md"} for path in (plugin / ".ai-rulez/commands").glob("*.md"))
    with tempfile.TemporaryDirectory(prefix="pi-package-") as destination:
        result = subprocess.run(
            ["npm", "pack", "--json", "--ignore-scripts", "--offline", "--pack-destination", destination],
            cwd=plugin,
            check=True,
            text=True,
            capture_output=True,
        )
        archive = Path(destination) / json.loads(result.stdout)[0]["filename"]
        verify_archive(archive, metadata["name"], skills, prompts)


if __name__ == "__main__":
    main()
