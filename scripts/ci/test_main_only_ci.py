"""Reject CI jobs that can run outside main, including manual dispatches."""

from __future__ import annotations

import ast
import re
import unittest
from pathlib import Path

import yaml

WORKFLOWS = Path(__file__).parents[2] / ".github" / "workflows"
MAIN_GUARD = ast.dump(ast.parse("github.ref == 'refs/heads/main'", mode="eval").body)


def requires_main(expression: str) -> bool:
    """Require a main-ref equality as an unconditional conjunct of the job condition."""
    expression = expression.removeprefix("${{").removesuffix("}}").strip()
    expression = expression.replace("&&", " and ").replace("||", " or ")
    expression = re.sub(r"!(?!=)", "not ", expression).strip()
    parsed = ast.parse(expression, mode="eval").body
    conjuncts = parsed.values if isinstance(parsed, ast.BoolOp) and isinstance(parsed.op, ast.And) else [parsed]
    return any(ast.dump(term) == MAIN_GUARD for term in conjuncts)


def ci_workflows() -> list[Path]:
    """Exclude publishing entrypoints, including the tagged plugin-asset publisher."""
    return sorted(
        path
        for path in WORKFLOWS.iterdir()
        if not path.name.startswith("publish") and path.name != "plugin-artifacts.yaml"
    )


def checks_out_main_source(step: dict[str, object]) -> bool:
    """Reject alternate source refs while retaining pinned sibling dependency checkouts."""
    if not str(step.get("uses", "")).startswith("actions/checkout@"):
        return True
    inputs = step.get("with", {})
    repository = inputs.get("repository", "")
    if repository and repository not in {"xberg-io/xberg", "${{ github.repository }}"}:
        return "${{" not in repository
    return inputs.get("ref", "") in {"", "${{ github.sha }}"}


def keeps_latest_ci_run(concurrency: object, workflow: str) -> bool:
    """Cancel superseded runs within one workflow's stable ref group."""
    return (
        isinstance(concurrency, dict)
        and concurrency.get("cancel-in-progress") is True
        and concurrency.get("group") == f"{workflow}-${{{{ github.ref }}}}"
    )


class MainOnlyCiTests(unittest.TestCase):
    def test_every_ci_workflow_keeps_only_its_latest_main_run(self) -> None:
        paths = ci_workflows()
        assert len(paths) == 15, "CI workflow inventory changed"
        assert len({path.stem for path in paths}) == 15, "CI workflow group prefixes collide"
        job_count = 0
        for path in paths:
            workflow = yaml.safe_load(path.read_text())
            with self.subTest(workflow=path.name):
                assert keeps_latest_ci_run(workflow.get("concurrency"), path.stem), (
                    f"{path}: require a stable workflow-ref group with cancellation enabled"
                )
            for name, job in workflow["jobs"].items():
                with self.subTest(workflow=path.name, job=name):
                    assert "concurrency" not in job, "job concurrency can replace other jobs within one workflow run"
                job_count += 1
        assert job_count == 73, "CI job inventory changed"
        print(f"Checked latest-only groups for {len(paths)} CI workflows and {job_count} jobs")

    def test_accepts_stable_workflow_ref_group_with_cancellation(self) -> None:
        assert keeps_latest_ci_run({"group": "ci-rust-${{ github.ref }}", "cancel-in-progress": True}, "ci-rust")

    def test_rejects_missing_concurrency_and_disabled_cancellation(self) -> None:
        assert not keeps_latest_ci_run(None, "ci-rust")
        for cancellation in [False, "true", None]:
            with self.subTest(cancellation=cancellation):
                assert not keeps_latest_ci_run(
                    {"group": "ci-rust-${{ github.ref }}", "cancel-in-progress": cancellation}, "ci-rust"
                )

    def test_rejects_run_specific_or_input_dependent_groups(self) -> None:
        for expression in [
            "github.sha",
            "github.run_id",
            "github.run_attempt",
            "inputs.group",
            "github.event.timestamp",
        ]:
            with self.subTest(expression=expression):
                group = f"ci-rust-${{{{ github.ref }}}}-${{{{ {expression} }}}}"
                assert not keeps_latest_ci_run({"group": group, "cancel-in-progress": True}, "ci-rust")

    def test_rejects_groups_shared_with_another_workflow(self) -> None:
        for group in ["ci-docs-${{ github.ref }}", "ci-${{ github.ref }}", "${{ github.ref }}"]:
            with self.subTest(group=group):
                assert not keeps_latest_ci_run({"group": group, "cancel-in-progress": True}, "ci-rust")

    def test_every_ci_job_requires_main_and_no_workflow_has_pr_triggers(self) -> None:
        paths = ci_workflows()
        assert paths, "no CI workflows discovered"
        assert WORKFLOWS / "ci-rust.yaml" in paths
        job_count = 0
        for path in paths:
            workflow = yaml.safe_load(path.read_text())
            triggers = workflow.get("on", workflow.get(True))
            assert not {"pull_request", "pull_request_target"}.intersection(triggers), path
            jobs = workflow["jobs"]
            assert jobs, f"{path}: no jobs discovered"
            for name, job in jobs.items():
                with self.subTest(workflow=path.name, job=name):
                    condition = job.get("if", "")
                    assert condition and requires_main(condition), f"{path}:{name}: main ref is not required"
                job_count += 1
        assert job_count >= len(paths), "workflow job inventory was incomplete"
        print(f"Checked {job_count} jobs across {len(paths)} CI workflows")

    def test_ci_checkouts_cannot_override_the_dispatched_main_revision(self) -> None:
        checkout_count = 0
        for path in ci_workflows():
            workflow = yaml.safe_load(path.read_text())
            for name, job in workflow["jobs"].items():
                for step in job.get("steps", []):
                    with self.subTest(workflow=path.name, job=name, step=step.get("name", step.get("uses"))):
                        assert checks_out_main_source(step), f"{path}:{name}: alternate source checkout"
                    checkout_count += str(step.get("uses", "")).startswith("actions/checkout@")
        assert checkout_count == 74, "CI checkout inventory changed"
        print(f"Checked {checkout_count} CI checkout steps")

    def test_rejects_checkout_ref_inputs_and_alternate_branches(self) -> None:
        for ref in ["${{ inputs.branch }}", "${{ github.event.inputs.branch || github.sha }}", "codex/example"]:
            with self.subTest(ref=ref):
                assert not checks_out_main_source({"uses": "actions/checkout@v7", "with": {"ref": ref}})

    def test_retains_pinned_external_dependency_checkouts(self) -> None:
        step = {"uses": "actions/checkout@v7", "with": {"repository": "xberg-io/crawlberg", "ref": "v1.0.0"}}
        assert checks_out_main_source(step)

    def test_accepts_main_guard_with_existing_job_conditions(self) -> None:
        conditions = [
            "github.ref == 'refs/heads/main'",
            "${{ github.ref == 'refs/heads/main' && (always() && !cancelled()) }}",
            "${{ needs.setup.result == 'success' && github.ref == 'refs/heads/main' }}",
        ]
        for condition in conditions:
            with self.subTest(condition=condition):
                assert requires_main(condition)

    def test_rejects_missing_ref_guard_even_with_branch_push_filter(self) -> None:
        assert not requires_main("github.repository == 'xberg-io/xberg'")

    def test_rejects_manual_dispatch_escape_from_main_guard(self) -> None:
        condition = "${{ github.ref == 'refs/heads/main' || github.event_name == 'workflow_dispatch' }}"
        assert not requires_main(condition)

    def test_rejects_main_guard_in_only_one_alternative(self) -> None:
        condition = "${{ (github.ref == 'refs/heads/main' && success()) || always() }}"
        assert not requires_main(condition)

    def test_rejects_branch_or_tag_guard_instead_of_main(self) -> None:
        for ref in ["refs/heads/codex/example", "refs/pull/42/merge", "refs/tags/v1.4.0"]:
            with self.subTest(ref=ref):
                assert not requires_main(f"github.ref == '{ref}'")


if __name__ == "__main__":
    unittest.main()
