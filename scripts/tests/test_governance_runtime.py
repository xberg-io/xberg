"""Check that ai-rulez is never pinned and always runs its latest release (org ADR-0015)."""

from pathlib import Path
import re
import tomllib
import unittest

SETUP_AI_RULEZ = "xberg-io/actions/setup-ai-rulez@v1"
# The reusable validation workflow installs ai-rulez through setup-ai-rulez itself.
REUSABLE_VALIDATION = "xberg-io/actions/.github/workflows/reusable-ai-rulez-validation.yml@v1"


def _latest_ai_rulez_installs(path: Path) -> int:
    source = path.read_text()
    assert "github:Goldziher/ai-rulez" not in source, f"{path} installs ai-rulez through mise"
    assert "ai-rulez-version:" not in source, f"{path} passes an ai-rulez version"
    assert "source-revision:" not in source, f"{path} builds a pinned ai-rulez revision"
    assert re.search(r"AI_RULEZ\w*_VERSION", source) is None, f"{path} pins ai-rulez through the environment"
    installers = "|".join(re.escape(ref) for ref in (SETUP_AI_RULEZ, REUSABLE_VALIDATION))
    return len(re.findall(rf"(?m)^ +(?:- )?uses: (?:{installers})$", source))


def _hook_revision(sources: list[dict]) -> str | None:
    found = [source for source in sources if source.get("id") == "ai-rulez"]
    assert len(found) == 1, found
    return found[0].get("revision")


class GovernanceRuntimeTests(unittest.TestCase):
    """No mise pin, no literal version, setup-ai-rulez in CI, npx in tasks and the hook source on v5."""

    root = Path(__file__).resolve().parents[2]

    def test_should_not_pin_ai_rulez_in_any_mise_config(self) -> None:
        """No mise config or task pins ai-rulez; tasks run it through npx at latest."""
        configs = sorted(self.root.glob("mise*.toml"))
        assert configs, self.root
        for path in configs:
            assert "github:Goldziher/ai-rulez" not in tomllib.loads(path.read_text()).get("tools", {}), path
        tasks = [path for path in (self.root / "mise-tasks").rglob("*") if path.is_file()]
        assert tasks, "mise-tasks/ holds no task files"
        for path in [*configs, *tasks]:
            source = path.read_text()
            assert re.search(r"AI_RULEZ\w*_VERSION", source) is None, f"{path} pins ai-rulez through the environment"
            assert re.search(r"ai-rulez@(?!latest\b)", source) is None, f"{path} runs a pinned ai-rulez"

    def test_should_install_the_latest_release_in_ci_and_release_workflows(self) -> None:
        """Two CI installs and six release installs use setup-ai-rulez, which takes the latest release."""
        release_paths = {"publish.yaml", "plugin-artifacts.yaml"}
        installs = {"ci": 0, "release": 0}
        for path in (self.root / ".github/workflows").glob("*"):
            installs["release" if path.name in release_paths else "ci"] += _latest_ai_rulez_installs(path)
        assert installs == {"ci": 2, "release": 6}, installs

    def test_should_follow_the_floating_major_tag_for_the_hook_source(self) -> None:
        """alef.toml and the poly.toml it renders take the ai-rulez hooks from v5."""
        alef = tomllib.loads((self.root / "alef.toml").read_text())
        poly = tomllib.loads((self.root / "poly.toml").read_text())
        assert _hook_revision(alef["workspace"]["poly"]["hooks-sources"]) == "v5"
        assert _hook_revision(poly["hooks"]["sources"]) == "v5"


if __name__ == "__main__":
    unittest.main()
