"""Reject CI jobs that can run outside main, including manual dispatches."""

from __future__ import annotations

import ast
import re
import unittest
from pathlib import Path
from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from collections.abc import Callable

WORKFLOWS = Path(__file__).parents[2] / ".github" / "workflows"
MAIN_GUARD = ast.dump(ast.parse("github.ref == 'refs/heads/main'", mode="eval").body)
EXPECTED_WORKFLOWS = 17
EXPECTED_JOBS = 77
EXPECTED_CHECKOUTS = 77
# The one sanctioned pull-request workflow: fixed regression tests, for same-repository branches only.
FOCUSED_PR_WORKFLOW = "ci-focused-pr.yaml"
FOCUSED_PR_GUARDS = (
    "github.repository == 'xberg-io/xberg'",
    "github.event.pull_request.head.repo.full_name == github.repository",
)


def requires_main(expression: str) -> bool:
    """Require a main-ref equality as an unconditional conjunct of the job condition."""
    expression = expression.removeprefix("${{").removesuffix("}}").strip()
    expression = expression.replace("&&", " and ").replace("||", " or ")
    expression = re.sub(r"!(?!=)", "not ", expression).strip()
    parsed = ast.parse(f"({expression})", mode="eval").body
    conjuncts = parsed.values if isinstance(parsed, ast.BoolOp) and isinstance(parsed.op, ast.And) else [parsed]
    return any(ast.dump(term) == MAIN_GUARD for term in conjuncts)


def ci_workflows() -> list[Path]:
    """Exclude publishing entrypoints, including the tagged plugin-asset publisher."""
    return sorted(
        path
        for path in WORKFLOWS.iterdir()
        if not path.name.startswith("publish") and path.name not in {"plugin-artifacts.yaml", FOCUSED_PR_WORKFLOW}
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


def focused_pr_violations(workflow: dict) -> list[str]:
    """Keep the pull-request exemption from widening into secrets, forks or superseded runs."""
    triggers = workflow.get("on", workflow.get(True))
    violations = []
    if set(triggers) != {"pull_request", "workflow_dispatch"}:
        violations.append(f"triggers: {sorted(triggers)}")
    if workflow.get("permissions") != {"contents": "read"}:
        violations.append("permissions must stay contents: read")
    if not keeps_latest_ci_run(workflow.get("concurrency"), Path(FOCUSED_PR_WORKFLOW).stem):
        violations.append("superseded runs must cancel")
    for name, job in workflow["jobs"].items():
        condition = job.get("if", "")
        violations.extend(f"{name}: missing {guard}" for guard in FOCUSED_PR_GUARDS if guard not in condition)
        for step in job["steps"]:
            is_checkout = str(step.get("uses", "")).startswith("actions/checkout@")
            if is_checkout and step.get("with", {}).get("persist-credentials") is not False:
                violations.append(f"{name}: checkout must not persist credentials")
    return violations


class MainOnlyCiTests(unittest.TestCase):
    def test_focused_pr_workflow_stays_read_only_and_same_repository(self) -> None:
        workflow = yaml.safe_load((WORKFLOWS / FOCUSED_PR_WORKFLOW).read_text())
        assert focused_pr_violations(workflow) == []

    def test_focused_pr_check_rejects_each_widening(self) -> None:
        def mutated(change: Callable[[dict], None]) -> dict:
            workflow = yaml.safe_load((WORKFLOWS / FOCUSED_PR_WORKFLOW).read_text())
            change(workflow)
            return workflow

        def add_target_trigger(workflow: dict) -> None:
            workflow[True]["pull_request_target"] = None

        def grant_write(workflow: dict) -> None:
            workflow["permissions"] = {"contents": "write"}

        def stop_cancelling(workflow: dict) -> None:
            workflow["concurrency"]["cancel-in-progress"] = False

        def admit_forks(workflow: dict) -> None:
            workflow["jobs"]["regressions"]["if"] = "${{ github.repository == 'xberg-io/xberg' }}"

        def persist_credentials(workflow: dict) -> None:
            next(
                step
                for step in workflow["jobs"]["regressions"]["steps"]
                if str(step.get("uses", "")).startswith("actions/checkout@")
            )["with"]["persist-credentials"] = True

        for change in [add_target_trigger, grant_write, stop_cancelling, admit_forks, persist_credentials]:
            with self.subTest(change=change.__name__):
                assert focused_pr_violations(mutated(change))

    def test_every_ci_workflow_keeps_only_its_latest_main_run(self) -> None:
        paths = ci_workflows()
        assert len(paths) == EXPECTED_WORKFLOWS, "CI workflow inventory changed"
        assert len({path.stem for path in paths}) == EXPECTED_WORKFLOWS, "CI workflow group prefixes collide"
        job_count = 0
        for path in paths:
            workflow = yaml.safe_load(path.read_text())
            with self.subTest(workflow=path.name):
                assert keeps_latest_ci_run(workflow.get("concurrency"), path.stem), (
                    f"{path}: require a stable workflow-ref group with the expected cancellation"
                )
            for name, job in workflow["jobs"].items():
                with self.subTest(workflow=path.name, job=name):
                    assert "concurrency" not in job, "job concurrency can replace other jobs within one workflow run"
                job_count += 1
        assert job_count == EXPECTED_JOBS, "CI job inventory changed"
        print(f"Checked latest-only groups for {len(paths)} CI workflows and {job_count} jobs")

    def test_accepts_stable_workflow_ref_group_with_cancellation(self) -> None:
        paths = ci_workflows()
        assert len(paths) == EXPECTED_WORKFLOWS, "CI workflow inventory changed"
        for path in paths:
            with self.subTest(workflow=path.name):
                group = f"{path.stem}-${{{{ github.ref }}}}"
                assert keeps_latest_ci_run({"group": group, "cancel-in-progress": True}, path.stem)
                assert not keeps_latest_ci_run({"group": group, "cancel-in-progress": False}, path.stem)

    def test_no_ci_workflow_queues_superseded_runs(self) -> None:
        paths = ci_workflows()
        assert len(paths) == EXPECTED_WORKFLOWS, "CI workflow inventory changed"
        for path in paths:
            with self.subTest(workflow=path.name):
                workflow = yaml.safe_load(path.read_text())
                assert workflow["concurrency"]["cancel-in-progress"] is True, f"{path}: superseded runs must cancel"

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
        assert checkout_count == EXPECTED_CHECKOUTS, "CI checkout inventory changed"
        print(f"Checked {checkout_count} CI checkout steps")

    def test_rejects_checkout_ref_inputs_and_alternate_branches(self) -> None:
        for ref in ["${{ inputs.branch }}", "${{ github.event.inputs.branch || github.sha }}", "codex/example"]:
            with self.subTest(ref=ref):
                assert not checks_out_main_source({"uses": "actions/checkout@v7", "with": {"ref": ref}})

    def test_retains_pinned_external_dependency_checkouts(self) -> None:
        step = {"uses": "actions/checkout@v7", "with": {"repository": "xberg-io/crawlberg", "ref": "v1.0.0"}}
        assert checks_out_main_source(step)

    def test_nli_advisories_remain_scheduled_main_only_without_model_builds(self) -> None:
        workflow = yaml.safe_load((WORKFLOWS / "ci-nli-advisories.yaml").read_text())
        triggers = workflow.get("on", workflow.get(True))
        assert set(triggers) == {"schedule", "workflow_dispatch"}
        assert triggers["schedule"] == [{"cron": "0 6 * * 1"}]
        assert workflow["permissions"] == {"contents": "read"}
        assert set(workflow["jobs"]) == {"audit"}
        job = workflow["jobs"]["audit"]
        assert requires_main(job["if"])
        assert not requires_main(job["if"].replace("github.ref == 'refs/heads/main'", "true"))
        commands = [step["run"] for step in job["steps"] if "run" in step]
        assert commands == [
            "mise run nli:audit -- --advisories",
            "mise run nli:audit:advisory-control",
        ]
        setup = [step for step in job["steps"] if step.get("uses") == "xberg-io/actions/setup-mise@v1"]
        assert len(setup) == 1 and setup[0]["with"]["install-args"] == "cargo-deny"
        assert not any("onnx" in str(step.get("uses", "")) for step in job["steps"])

    def test_accepts_main_guard_with_existing_job_conditions(self) -> None:
        conditions = [
            "github.ref == 'refs/heads/main'",
            "${{ github.ref == 'refs/heads/main' && (always() && !cancelled()) }}",
            "${{ needs.setup.result == 'success' && github.ref == 'refs/heads/main' }}",
            "${{ !cancelled() &&\n  github.ref == 'refs/heads/main' &&\n  (success() || failure()) }}",
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
        multiline_condition = "${{ (github.ref == 'refs/heads/main' && success()) ||\n  always() }}"
        assert not requires_main(multiline_condition)

    def test_rejects_branch_or_tag_guard_instead_of_main(self) -> None:
        for ref in ["refs/heads/codex/example", "refs/pull/42/merge", "refs/tags/v1.4.0"]:
            with self.subTest(ref=ref):
                assert not requires_main(f"github.ref == '{ref}'")


if __name__ == "__main__":
    unittest.main()
