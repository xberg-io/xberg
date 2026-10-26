"""Exercise the root package.json Pi resource check against real and broken trees."""

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path


def assert_error(operation: Callable[[], object], message: str) -> None:
    """Require the intended rejection instead of any failure."""
    try:
        operation()
    except ValueError as error:
        received = str(error)
    else:
        raise AssertionError(f"Expected rejection: {message}")
    assert message in received


def load_module() -> object:
    """Load the repository script by path."""
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("verify_pi_package", root / "scripts/verify_pi_package.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RootPiResourcesTests(unittest.TestCase):
    """Build a minimal root and plugin tree for every case."""

    def setUp(self) -> None:
        """Create one skill and one prompt that both package.json files name."""
        self.module = load_module()
        directory = tempfile.TemporaryDirectory(prefix="pi-root-contract-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root / "plugin/.pi/skills/demo").mkdir(parents=True)
        (self.root / "plugin/.pi/skills/demo/SKILL.md").write_text("Body\n")
        (self.root / "plugin/.pi/prompts").mkdir(parents=True)
        (self.root / "plugin/.pi/prompts/demo.md").write_text("Body\n")
        self.write_root(["./plugin/.pi/skills/demo"], ["./plugin/.pi/prompts/demo.md"])
        self.write_plugin(["./.pi/skills/demo"], ["./.pi/prompts/demo.md"])

    def write_root(self, skills: list[str], prompts: list[str]) -> None:
        """Write the root package.json."""
        (self.root / "package.json").write_text(json.dumps({"pi": {"skills": skills, "prompts": prompts}}))

    def write_plugin(self, skills: list[str], prompts: list[str]) -> None:
        """Write the plugin package.json."""
        (self.root / "plugin/package.json").write_text(json.dumps({"pi": {"skills": skills, "prompts": prompts}}))

    def test_matching_root_passes(self) -> None:
        """A root list that resolves and matches the plugin list is accepted."""
        self.module.verify_root_pi(self.root)

    def test_missing_skill_directory_is_rejected(self) -> None:
        """A skill path with no directory fails."""
        self.write_root(["./plugin/.pi/skills/gone"], ["./plugin/.pi/prompts/demo.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "gone")

    def test_skill_directory_without_skill_file_is_rejected(self) -> None:
        """A skill directory with no SKILL.md fails."""
        (self.root / "plugin/.pi/skills/demo/SKILL.md").unlink()
        assert_error(lambda: self.module.verify_root_pi(self.root), "demo")

    def test_missing_prompt_file_is_rejected(self) -> None:
        """A prompt path with no file fails."""
        self.write_root(["./plugin/.pi/skills/demo"], ["./plugin/.pi/prompts/gone.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "gone")

    def test_skill_names_must_match_plugin(self) -> None:
        """A root list that drops a plugin skill fails even when every path resolves."""
        (self.root / "plugin/.pi/skills/extra").mkdir()
        (self.root / "plugin/.pi/skills/extra/SKILL.md").write_text("Body\n")
        self.write_plugin(["./.pi/skills/demo", "./.pi/skills/extra"], ["./.pi/prompts/demo.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "extra")

    def test_prompt_directory_is_rejected(self) -> None:
        """A directory at a prompt path fails, because pi loads a prompt file."""
        (self.root / "plugin/.pi/prompts/folder.md").mkdir()
        self.write_root(["./plugin/.pi/skills/demo"], ["./plugin/.pi/prompts/folder.md"])
        self.write_plugin(["./.pi/skills/demo"], ["./.pi/prompts/folder.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "folder.md")

    def test_root_drops_a_plugin_prompt(self) -> None:
        """A root list that drops a plugin prompt fails even when every path resolves."""
        (self.root / "plugin/.pi/prompts/extra.md").write_text("Body\n")
        self.write_plugin(["./.pi/skills/demo"], ["./.pi/prompts/demo.md", "./.pi/prompts/extra.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "extra.md")

    def test_root_adds_a_prompt_the_plugin_lacks(self) -> None:
        """A root prompt that the plugin does not list fails."""
        (self.root / "plugin/.pi/prompts/extra.md").write_text("Body\n")
        self.write_root(["./plugin/.pi/skills/demo"], ["./plugin/.pi/prompts/demo.md", "./plugin/.pi/prompts/extra.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "extra.md")

    def test_root_adds_a_skill_the_plugin_lacks(self) -> None:
        """A root skill that the plugin does not list fails."""
        (self.root / "plugin/.pi/skills/extra").mkdir()
        (self.root / "plugin/.pi/skills/extra/SKILL.md").write_text("Body\n")
        self.write_root(["./plugin/.pi/skills/demo", "./plugin/.pi/skills/extra"], ["./plugin/.pi/prompts/demo.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "extra")

    def test_duplicate_skill_name_is_rejected(self) -> None:
        """A skill listed twice fails, even through an alias path."""
        self.write_root(
            ["./plugin/.pi/skills/demo", "./plugin/.pi/skills/../skills/demo"], ["./plugin/.pi/prompts/demo.md"]
        )
        assert_error(lambda: self.module.verify_root_pi(self.root), "twice")

    def test_duplicate_prompt_name_is_rejected(self) -> None:
        """A prompt listed twice fails."""
        self.write_root(["./plugin/.pi/skills/demo"], ["./plugin/.pi/prompts/demo.md", "./plugin/.pi/prompts/demo.md"])
        assert_error(lambda: self.module.verify_root_pi(self.root), "twice")

    def test_script_exits_nonzero_on_a_broken_root(self) -> None:
        """The script itself, which CI runs, stops on a bad root list before it packs anything."""
        (self.root / "scripts").mkdir()
        shutil.copy(Path(__file__).resolve().parents[1] / "verify_pi_package.py", self.root / "scripts")
        self.write_root(["./plugin/.pi/skills/gone"], ["./plugin/.pi/prompts/demo.md"])
        result = subprocess.run(
            [sys.executable, "-I", str(self.root / "scripts/verify_pi_package.py")],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 1
        assert "Root package.json pi.skills path does not resolve" in result.stderr

    def test_real_repository_passes(self) -> None:
        """The checked-in root package.json passes."""
        self.module.verify_root_pi(Path(__file__).resolve().parents[2])


if __name__ == "__main__":
    unittest.main()
