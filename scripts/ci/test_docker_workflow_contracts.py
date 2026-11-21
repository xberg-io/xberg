"""Contract tests for the image notice gate in the Docker CI and publish workflows.

A text lint over the Dockerfiles cannot prove that a shipped image holds the
current notices. These tests assert that both workflows open the built image
with `verify_image_notices.py` before it can be tested or pushed, that the
matrix covers the cli image, and that editing a notice source triggers the
checks.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

ROOT = Path(__file__).resolve().parents[2]
CI_DOCKER = ROOT / ".github" / "workflows" / "ci-docker.yaml"
PUBLISH_DOCKER = ROOT / ".github" / "workflows" / "publish-docker.yaml"
CI_LINT = ROOT / ".github" / "workflows" / "ci-lint.yaml"
VERIFIER = "scripts/ci/docker/verify_image_notices.py"
NOTICE_SOURCES = ("LICENSE", "THIRD_PARTY_LICENSES.md")
NOTICE_COPY = "COPY LICENSE THIRD_PARTY_LICENSES.md /usr/local/share/licenses/xberg/"


def load_workflow(text: str) -> dict[str, object]:
    """Parse workflow text, keeping the unquoted `on:` trigger key intact."""
    workflow = yaml.safe_load(text)
    if "on" not in workflow and True in workflow:
        workflow["on"] = workflow[True]
    return workflow


def push_paths(text: str) -> list[str]:
    """Return the push path filters, failing when the workflow is not path-filtered."""
    triggers = load_workflow(text)["on"]
    assert isinstance(triggers, dict) and "push" in triggers, "workflow has no push trigger"
    return triggers["push"]["paths"]


def named_step(steps: Sequence[dict[str, object]], name: str) -> dict[str, object]:
    """Return the single step with this name."""
    matches = [step for step in steps if step.get("name") == name]
    assert len(matches) == 1, f"expected one {name!r} step, found {len(matches)}"
    return matches[0]


def step_names(steps: Sequence[dict[str, object]]) -> list[str]:
    """Return the step names in order."""
    return [str(step.get("name")) for step in steps]


def assert_ci_docker_verifies_notices(text: str) -> None:
    """The CI matrix must open every final image, cli included, right after its build."""
    job = load_workflow(text)["jobs"]["docker"]
    assert job["strategy"]["matrix"]["variant"] == ["core", "full", "cli"]
    steps = job["steps"]
    names = step_names(steps)
    verifier = named_step(steps, "Verify image notices")
    assert "if" not in verifier, "the notice gate must not exclude the cli image"
    assert VERIFIER in str(verifier.get("run"))
    assert "xberg:${{ matrix.variant }}" in str(verifier.get("run"))
    assert names.index("Verify image notices") == names.index("Build Docker image") + 1


def assert_ci_docker_paths_cover_notices(text: str) -> None:
    """Editing a notice source or the gate itself must trigger the image check."""
    paths = push_paths(text)
    for entry in (*NOTICE_SOURCES, "scripts/ci/docker/**", ".github/workflows/ci-docker.yaml"):
        assert entry in paths, f"ci-docker.yaml path filters omit {entry}"


def assert_publish_docker_verifies_before_push(text: str) -> None:
    """A digest must not be pushable while the test image's notices are stale."""
    job = load_workflow(text)["jobs"]["build-docker"]
    steps = job["steps"]
    names = step_names(steps)
    verifier = named_step(steps, "Verify image notices")
    assert names.index("Verify image notices") > names.index("Run Docker tests")
    assert names.index("Verify image notices") < names.index("Build and push by digest")
    assert verifier.get("if") == "${{ steps.gate.outputs.go == 'true' }}"
    assert VERIFIER in str(verifier.get("run"))
    assert "xberg-publish:${{ matrix.variant }}-test" in str(verifier.get("run"))


def assert_every_published_variant_copies_notices(text: str) -> None:
    """Every variant the workflow can push must carry the canonical notice pair."""
    includes = load_workflow(text)["jobs"]["build-docker"]["strategy"]["matrix"]["include"]
    assert {entry["variant"] for entry in includes} == {"core", "full", "cli"}
    for entry in includes:
        dockerfile = ROOT / entry["dockerfile"]
        assert NOTICE_COPY in dockerfile.read_text(), f"{dockerfile} does not copy both notices"


def assert_ci_lint_paths_cover_notices(text: str) -> None:
    """The workflow that runs these contracts must run when a notice source changes."""
    paths = push_paths(text)
    for entry in NOTICE_SOURCES:
        assert entry in paths, f"ci-lint.yaml path filters omit {entry}"


def remove_step(text: str, name: str) -> str:
    """Delete a step and its indented body, for the negative controls."""
    pattern = rf"(?m)^      - name: {re.escape(name)}\n(?:        .*\n)*"
    return re.sub(pattern, "", text, count=1)


def assert_rejected(check: Callable[[str], None], workflow: str, reason: str) -> None:
    """Fail unless the contract rejects this workflow text."""
    try:
        check(workflow)
    except AssertionError:
        return
    raise AssertionError(f"contract accepted a workflow with {reason}")


def test_ci_docker_runs_the_byte_level_verifier_for_every_variant() -> None:
    assert_ci_docker_verifies_notices(CI_DOCKER.read_text())


def test_ci_docker_path_filters_cover_the_notice_sources() -> None:
    assert_ci_docker_paths_cover_notices(CI_DOCKER.read_text())


def test_publish_docker_verifies_notice_bytes_before_pushing_a_digest() -> None:
    assert_publish_docker_verifies_before_push(PUBLISH_DOCKER.read_text())


def test_every_published_variant_keeps_the_canonical_notice_copy() -> None:
    assert_every_published_variant_copies_notices(PUBLISH_DOCKER.read_text())


def test_ci_lint_path_filters_cover_the_notice_sources() -> None:
    assert_ci_lint_paths_cover_notices(CI_LINT.read_text())


def test_ci_contract_rejects_a_workflow_without_the_verifier() -> None:
    broken = remove_step(CI_DOCKER.read_text(), "Verify image notices")

    assert_rejected(assert_ci_docker_verifies_notices, broken, "no image notice verifier")


def test_publish_contract_rejects_a_workflow_without_the_verifier() -> None:
    broken = remove_step(PUBLISH_DOCKER.read_text(), "Verify image notices")

    assert_rejected(assert_publish_docker_verifies_before_push, broken, "no image notice verifier")


def test_ci_docker_contract_rejects_omitted_notice_sources() -> None:
    broken = CI_DOCKER.read_text().replace('      - "LICENSE"\n', "")

    assert_rejected(assert_ci_docker_paths_cover_notices, broken, "no LICENSE path filter")


if __name__ == "__main__":
    for name, test in list(globals().items()):
        if name.startswith("test_") and callable(test):
            test()
