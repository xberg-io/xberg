"""Contract tests for the image notice gate in the Docker CI and publish workflows.

A text lint over the Dockerfiles cannot prove that a shipped image holds the
current notices. These tests assert that both workflows open the built image
with `verify_image_notices.py` before it can be tested or pushed, that the
matrix covers the cli image, and that editing a notice source triggers the
checks.
"""

from __future__ import annotations

import fnmatch
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


def assert_publish_docker_verifies_best_before_push(text: str) -> None:
    """The tessdata_best twin is tested and notice-checked before its digest can be pushed."""
    steps = load_workflow(text)["jobs"]["build-docker"]["steps"]
    names = step_names(steps)
    verifier = named_step(steps, "Verify best image notices")
    assert names.index("Run best Docker tests") < names.index("Verify best image notices")
    assert names.index("Verify best image notices") < names.index("Build and push best by digest")
    assert names.index("Upload digest") < names.index("Build best test image"), (
        "best must not hold back the default image"
    )
    assert verifier.get("if") == "${{ matrix.best && steps.gate.outputs.go_best == 'true' }}"
    assert VERIFIER in str(verifier.get("run"))
    assert "xberg-publish:${{ matrix.variant }}-best-test" in str(verifier.get("run"))
    tests = named_step(steps, "Run best Docker tests")
    assert "--variant ${{ matrix.variant }}-best" in str(tests.get("run"))
    default_build = named_step(steps, "Build test image")["with"]
    for name in ("Build best test image", "Build and push best by digest"):
        build = named_step(steps, name)["with"]
        assert build.get("target") == "best", name
        assert build["build-args"] == default_build["build-args"], f"{name} must reuse the default build's cache"
        assert build["cache-from"] == default_build["cache-from"], name


def digest_artifacts(workflow: dict[str, object]) -> list[str]:
    """Expand the digest artifact names build-docker uploads for every matrix leg."""
    job = workflow["jobs"]["build-docker"]
    templates = [
        (step["with"]["name"], "best" in step["name"])
        for step in job["steps"]
        if str(step.get("uses", "")).startswith("actions/upload-artifact")
    ]
    assert len(templates) == 2, templates
    names = []
    for entry in job["strategy"]["matrix"]["include"]:
        for platform in job["strategy"]["matrix"]["platform"]:
            for template, best in templates:
                if best and not entry["best"]:
                    continue
                names.append(
                    template.replace("${{ matrix.platform.name }}", platform["name"]).replace(
                        "${{ matrix.variant }}", entry["variant"]
                    )
                )
    return names


def assert_merge_patterns_select_one_variant(text: str) -> None:
    """Each published manifest must collect exactly its own two platform digests."""
    workflow = load_workflow(text)
    artifacts = digest_artifacts(workflow)
    merge = workflow["jobs"]["merge-docker"]
    pattern = named_step(merge["steps"], "Download digests")["with"]["pattern"]
    variants = [entry["variant"] for entry in merge["strategy"]["matrix"]["include"]]
    assert sorted(variants) == ["cli", "core", "core-best", "full", "full-best"]
    for variant in variants:
        matched = fnmatch.filter(artifacts, pattern.replace("${{ matrix.variant }}", variant))
        assert len(matched) == 2, (variant, matched)
    assert len(artifacts) == 10, artifacts


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


def test_publish_docker_verifies_the_best_image_before_pushing_its_digest() -> None:
    assert_publish_docker_verifies_best_before_push(PUBLISH_DOCKER.read_text())


def test_each_manifest_downloads_only_its_own_digests() -> None:
    assert_merge_patterns_select_one_variant(PUBLISH_DOCKER.read_text())


def test_publish_contract_rejects_a_best_image_without_the_verifier() -> None:
    broken = remove_step(PUBLISH_DOCKER.read_text(), "Verify best image notices")

    assert_rejected(assert_publish_docker_verifies_best_before_push, broken, "no best image notice verifier")


def test_merge_contract_rejects_a_pattern_that_also_matches_best_digests() -> None:
    broken = PUBLISH_DOCKER.read_text().replace(
        "pattern: digest-*-${{ matrix.variant }}", "pattern: digest-*-${{ matrix.variant }}*"
    )

    assert_rejected(assert_merge_patterns_select_one_variant, broken, "an overlapping digest pattern")


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
