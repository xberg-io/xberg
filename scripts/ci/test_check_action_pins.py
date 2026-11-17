"""Exercise the offline action-pin policy on workflow and action manifests."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-action-pins.py")
SPEC = importlib.util.spec_from_file_location("check_action_pins", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)

SHA = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW = Path(".github/workflows/test.yaml")
QUALIFICATION = Path(".github/workflows/qualify-macos-elixir-nif.yaml")
QUALIFIED_SHA = "97c04280c1d4acc16bdc456db188782afbe63803"


class TestOfflinePinPolicy(unittest.TestCase):
    def test_checks_reusable_workflows_and_aliased_steps(self) -> None:
        text = f"""jobs:
  reusable:
    uses: outside/actions/.github/workflows/gate.yml@{SHA}
  primary: &testing
    env: {{ uses: documentation/input@main }}
    steps:
      - {{ "uses": "actions/checkout@{SHA}" }}
      - uses: xberg-io/actions/setup-rust@v1
      - uses: ./.github/actions/test
      - uses: $/.github/actions/test
      - uses: docker://alpine:3.23
      - run: |
          echo uses: example/action@main
  reused: *testing
"""
        assert CHECKER.check_manifest(WORKFLOW, text) == (3, [])

    def test_a_merged_step_cannot_hide_a_mutable_ref(self) -> None:
        text = """jobs:
  test:
    env: &input { uses: outside/action@main }
    steps:
      - <<: *input
"""
        assert CHECKER.check_manifest(WORKFLOW, text) == (
            1,
            [f"{WORKFLOW}:3: 'outside/action@main' must use a full 40-character commit SHA"],
        )

    def test_rejects_third_party_tags_branches_and_abbreviated_shas(self) -> None:
        for ref in ["v7", "v7.1.2", "main", SHA[:39], f"{SHA}0", f"{SHA[:39]}g"]:
            with self.subTest(ref=ref):
                text = f"jobs:\n  build:\n    steps:\n      - uses: actions/checkout@{ref}\n"
                count, errors = CHECKER.check_manifest(WORKFLOW, text)
                assert count == 1
                assert errors == [f"{WORKFLOW}:4: 'actions/checkout@{ref}' must use a full 40-character commit SHA"]

    def test_rejects_a_tag_on_a_reusable_workflow(self) -> None:
        text = "jobs:\n  call:\n    uses: outside/actions/.github/workflows/gate.yml@v1\n"
        count, errors = CHECKER.check_manifest(WORKFLOW, text)
        assert count == 1
        assert errors == [
            f"{WORKFLOW}:3: 'outside/actions/.github/workflows/gate.yml@v1' must use a full 40-character commit SHA"
        ]

    def test_scopes_first_party_sha_exceptions_to_the_existing_qualification(self) -> None:
        for action in ["build-elixir-natives", "setup-mise", "setup-rust"]:
            with self.subTest(action=action):
                text = f"jobs:\n  test:\n    steps:\n      - uses: xberg-io/actions/{action}@{QUALIFIED_SHA}\n"
                assert CHECKER.check_manifest(QUALIFICATION, text) == (1, [])
                count, errors = CHECKER.check_manifest(WORKFLOW, text)
                assert count == 1
                assert "must use a bare major tag" in errors[0]

        for action, ref in [
            ("setup-rust", SHA),
            ("new-action", QUALIFIED_SHA),
            ("setup-rust", "main"),
            ("setup-rust", "v1.2.3"),
        ]:
            with self.subTest(action=action, ref=ref):
                text = f"jobs:\n  test:\n    steps:\n      - uses: xberg-io/actions/{action}@{ref}\n"
                count, errors = CHECKER.check_manifest(QUALIFICATION, text)
                assert count == 1
                assert "must use a bare major tag" in errors[0]

    def test_checks_only_executable_steps_in_composite_actions(self) -> None:
        path = Path("packages/example/action.yaml")
        composite = f"""inputs: {{ uses: {{ default: ignored/repository@main }} }}
runs:
  using: composite
  steps:
    - {{ uses: "third-party/action@{SHA}" }}
    - uses: another/action@main
"""
        assert CHECKER.check_manifest(path, composite) == (
            2,
            [f"{path}:6: 'another/action@main' must use a full 40-character commit SHA"],
        )
        docker = "runs: { using: docker, image: 'docker://alpine:3.23' }\n"
        assert CHECKER.check_manifest(path, docker) == (0, [])

    def test_reports_duplicate_keys_and_non_scalar_action_refs(self) -> None:
        broken_inputs = [
            ("    steps:\n      - uses: xberg-io/actions/a@v1\n        uses: outside/act@main", "duplicate field"),
            ("    steps:\n      - uses: [outside/act@main]", "uses must be a scalar"),
        ]
        for body, message in broken_inputs:
            with self.subTest(message=message):
                try:
                    CHECKER.check_manifest(WORKFLOW, "jobs:\n  test:\n" + body)
                except ValueError as error:
                    problem = str(error)
                else:
                    self.fail("invalid workflow structure passed")
                assert message in problem

    def test_cli_reads_tracked_actions_outside_dot_github(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.git(root, "init", "--quiet")
            action = root / "packages/example action/action.yml"
            action.parent.mkdir(parents=True)
            action.write_text(
                f"runs: {{ using: composite, steps: [{{ uses: actions/checkout@{SHA} }}] }}\n", encoding="utf-8"
            )
            self.git(root, "add", "--", "packages/example action/action.yml")
            completed = self.run_checker(root)
            assert completed.returncode == 0, completed.stderr
            assert "Checked 1 remote uses reference(s) across 1 manifest(s)" in completed.stdout

            action.write_text("runs: { using: composite, steps: [{ uses: actions/checkout@v7 }] }\n", encoding="utf-8")
            completed = self.run_checker(root)
            assert completed.returncode == 1
            assert "packages/example action/action.yml:1: 'actions/checkout@v7' must use" in completed.stderr

    def test_cli_fails_when_it_checked_no_remote_refs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.git(root, "init", "--quiet")
            workflow = root / WORKFLOW
            workflow.parent.mkdir(parents=True)
            workflow.write_text("jobs: { test: { steps: [{ run: echo hello }] } }\n", encoding="utf-8")
            self.git(root, "add", "--", WORKFLOW.as_posix())
            completed = self.run_checker(root)
            assert completed.returncode == 1
            assert "checked no remote uses references" in completed.stderr

    @staticmethod
    def git(root: Path, *arguments: str) -> None:
        subprocess.run(["git", "-C", str(root), *arguments], check=True, capture_output=True)

    @staticmethod
    def run_checker(root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root)],
            capture_output=True,
            text=True,
            check=False,
        )


if __name__ == "__main__":
    unittest.main()
