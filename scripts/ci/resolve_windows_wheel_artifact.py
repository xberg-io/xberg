"""Resolve a reusable Windows wheel from the same main-branch smoke workflow."""

import json
import os
import re
import subprocess
from pathlib import Path

WORKFLOW = ".github/workflows/ci-python-wheel-smoke.yaml"
WINDOWS_JOB = "Build + smoke-test Python wheel (windows-2022)"
UPLOAD_STEP = "Upload Windows wheel for isolated qualification"


def producer_source(repository: str, run_id: int, run: dict) -> str:
    source = run.get("head_sha", "")
    if (
        run.get("id") != run_id
        or run.get("head_branch") != "main"
        or run.get("event") != "workflow_dispatch"
        or run.get("status") != "completed"
        or run.get("path") != WORKFLOW
        or run.get("repository", {}).get("full_name") != repository
        or (run.get("head_repository") or {}).get("full_name") != repository
        or not re.fullmatch(r"[0-9a-f]{40}", source)
    ):
        raise ValueError("invalid Windows wheel producer run")
    return source


def select_prior_artifact(repository: str, run_id: int, run: dict, jobs: list[dict], artifacts: list[dict]) -> dict:
    source = producer_source(repository, run_id, run)
    repo_id = run["repository"]["id"]
    name = f"clean-host-windows-wheel-{source}"
    matches = [artifact for artifact in artifacts if artifact.get("name") == name]
    if len(matches) != 1:
        raise ValueError(f"invalid artifact provenance: expected one {name}, got {len(matches)}")
    artifact = matches[0]
    identity = {
        "id": run_id,
        "repository_id": repo_id,
        "head_repository_id": repo_id,
        "head_branch": "main",
        "head_sha": source,
    }
    if (
        artifact.get("expired") is not False
        or not re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest") or "")
        or type(artifact.get("id")) is not int
        or artifact["id"] <= 0
        or any((artifact.get("workflow_run") or {}).get(key) != value for key, value in identity.items())
    ):
        raise ValueError("invalid artifact provenance: wrong identity, missing digest, or expired")
    created = artifact.get("created_at") or ""
    successful_uploads = [
        step
        for job in jobs
        if job.get("name") == WINDOWS_JOB and job.get("conclusion") == "success"
        for step in job.get("steps", [])
        if step.get("name") == UPLOAD_STEP
        and step.get("conclusion") == "success"
        and (step.get("started_at") or "~") <= created <= (step.get("completed_at") or "")
    ]
    if len(successful_uploads) != 1:
        raise ValueError("artifact must originate in exactly one successful Windows upload")
    return artifact


def github_api(endpoint: str, *, paginate: bool = False) -> dict | list:
    flags = ["--paginate", "--slurp"] if paginate else []
    result = subprocess.run(["gh", "api", *flags, endpoint], capture_output=True, text=True, check=True, timeout=120)
    return json.loads(result.stdout)


def main() -> None:
    repository = os.environ["GITHUB_REPOSITORY"]
    raw_id = os.environ["WHEEL_PRODUCER_RUN_ID"]
    if os.environ["WHEEL_PLATFORM"] != "windows" or not re.fullmatch(r"[1-9][0-9]{0,19}", raw_id):
        raise ValueError("reuse requires platform=windows and a positive decimal producer run ID")
    run_id = int(raw_id)
    endpoint = f"repos/{repository}/actions/runs/{run_id}"
    run = github_api(endpoint)
    source = producer_source(repository, run_id, run)
    jobs = [job for page in github_api(f"{endpoint}/jobs?filter=all", paginate=True) for job in page["jobs"]]
    artifacts = [
        artifact
        for page in github_api(f"{endpoint}/artifacts?name=clean-host-windows-wheel-{source}", paginate=True)
        for artifact in page["artifacts"]
    ]
    artifact = select_prior_artifact(repository, run_id, run, jobs, artifacts)
    with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
        print(f"source_sha={source}\nartifact_id={artifact['id']}", file=output)
    provenance = {"run_id": run_id, "source_sha": source, "artifact_id": artifact["id"], "digest": artifact["digest"]}
    Path("windows-wheel-artifact.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
