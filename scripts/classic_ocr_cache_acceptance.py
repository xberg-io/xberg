#!/usr/bin/env python3
"""Run the air-gapped classic OCR deployment-cache acceptance test.

This is intentionally an opt-in integration harness: it downloads roughly the
complete PaddleOCR and Sceptre catalogs on its first seed and performs real
inference with every supported recognition family.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import NamedTuple

EXPECTED_ARTIFACTS = 46
EXPECTED_CATALOG_IDS = 31
EXPECTED_PADDLE_ARTIFACTS = 37
EXPECTED_PADDLE_CATALOG_IDS = 22
EXPECTED_SCEPTRE_ARTIFACTS = 9
EXPECTED_SCEPTRE_CATALOG_IDS = 9
EN_MOBILE_MODEL_SHA256 = "70b2450eed39599af6b996c27a2f1a0ef30eeb49f9f66dd3e74f28f652befc89"
EN_MOBILE_DICTIONARY_SHA256 = "854c6bb3e5a9a8ceac81fa700927e86a8da0e9b329a2846c57fc686be9db93e5"
FAST_EMBEDDING_REVISION = "4b127809f88a5aa1569d1238032b5ff40e5879bc"
FAST_EMBEDDING_MODEL_FILE = "all-MiniLM-L6-v2/model_quantized.onnx"
FAST_EMBEDDING_MODEL_SHA256 = "afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1"


class AcceptanceError(RuntimeError):
    pass


class OcrJob(NamedTuple):
    name: str
    language: str
    fixture: str


class PaddleJob(NamedTuple):
    name: str
    language: str
    fixture: str
    model_version: str
    model_tier: str
    use_angle_cls: bool = False
    auto_rotate: bool = False


PADDLE_JOBS = (
    PaddleJob("english", "eng", "images/english.png", "pp-ocrv5", "mobile", True, True),
    PaddleJob("chinese", "zho", "images/chinese.jpg", "pp-ocrv5", "mobile"),
    PaddleJob("latin", "fra", "images/french.jpg", "pp-ocrv5", "mobile"),
    PaddleJob("korean", "kor", "images/korean.png", "pp-ocrv5", "mobile"),
    PaddleJob("eslav", "rus", "images/cyrillic.png", "pp-ocrv5", "mobile"),
    PaddleJob("thai", "tha", "images/english.png", "pp-ocrv5", "mobile"),
    PaddleJob("greek", "ell", "images/english.png", "pp-ocrv5", "mobile"),
    PaddleJob("arabic", "ara", "images/rtl_arabic_300dpi.png", "pp-ocrv5", "mobile"),
    PaddleJob("devanagari", "hin", "images/english.png", "pp-ocrv5", "mobile"),
    PaddleJob("tamil", "tam", "images/english.png", "pp-ocrv5", "mobile"),
    PaddleJob("telugu", "tel", "images/telugu.png", "pp-ocrv5", "mobile"),
    PaddleJob("english_server", "eng", "images/english.png", "pp-ocrv5", "server"),
    PaddleJob("v6_medium", "eng", "images/english.png", "pp-ocrv6", "medium"),
    PaddleJob("v6_small", "eng", "images/english.png", "pp-ocrv6", "small"),
    PaddleJob("v6_tiny", "eng", "images/english.png", "pp-ocrv6", "tiny"),
)

SCEPTRE_JOBS = (
    OcrJob("english", "eng", "images/english.png"),
    OcrJob("latin", "fra", "images/french.jpg"),
    OcrJob("simplified_chinese", "zho", "images/chinese.jpg"),
    OcrJob("japanese", "jpn", "images/japanese.jpg"),
    OcrJob("korean", "kor", "images/korean.png"),
    OcrJob("telugu", "tel", "images/telugu.png"),
    OcrJob("kannada", "kan", "images/kannada.png"),
    OcrJob("cyrillic", "rus", "images/cyrillic.png"),
)

MISSING_CACHE_JOBS = (
    ("paddle-ocr", PADDLE_JOBS[0]),
    ("sceptre", SCEPTRE_JOBS[0]),
)


def fail(message: str) -> None:
    raise AcceptanceError(message)


def validate_cache_root(path: Path, *, raw_value: str | None = None) -> Path:
    if raw_value == "":
        fail("cache root must not be empty")
    resolved = path.expanduser().resolve()
    if str(resolved) in {"/", str(Path.home().resolve())}:
        fail(f"refusing unsafe cache root: {resolved}")
    return resolved


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def snapshot_tree(root: Path) -> dict[str, tuple[str, int, int, str]]:
    if not root.is_dir():
        fail(f"cache root does not exist: {root}")
    snapshot: dict[str, tuple[str, int, int, str]] = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        stat = path.lstat()
        if path.is_symlink():
            snapshot[relative] = ("symlink", stat.st_size, stat.st_mtime_ns, str(path.readlink()))
        elif path.is_file():
            snapshot[relative] = ("file", stat.st_size, stat.st_mtime_ns, sha256(path))
        elif path.is_dir():
            snapshot[relative] = ("dir", 0, stat.st_mtime_ns, "")
    if not snapshot:
        fail("cache tree snapshot matched zero entries")
    return snapshot


def assert_tree_unchanged(first: dict, second: dict) -> None:
    if first != second:
        fail("cache tree changed during the second seed")


def is_lower_hex(value: object, length: int) -> bool:
    return (
        isinstance(value, str) and len(value) == length and all(character in "0123456789abcdef" for character in value)
    )


def validate_manifest_row(model: dict, index: int, cache_root: Path) -> None:
    for field in ("repo", "file", "catalog_id", "role", "artifact_kind"):
        if not isinstance(model.get(field), str) or not model[field]:
            fail(f"manifest model {index} has an empty {field}")
    if not is_lower_hex(model.get("revision"), 40):
        fail(f"manifest model {index} revision is not a 40-character lowercase hex commit")
    if not is_lower_hex(model.get("sha256"), 64):
        fail(f"manifest model {index} SHA-256 is not 64-character lowercase hex")
    if model.get("license") != "Apache-2.0":
        fail(f"manifest model {index} license: expected Apache-2.0, got {model.get('license')}")
    if not isinstance(model.get("size_bytes"), int) or model["size_bytes"] <= 0:
        fail(f"manifest model {index} size_bytes must be positive")
    path = Path(model.get("path", ""))
    try:
        resolved_path = path.resolve(strict=True)
        resolved_path.relative_to(cache_root)
    except (FileNotFoundError, ValueError):
        fail(f"manifest model {index} path is absent or outside the cache root: {path}")
    if not resolved_path.is_file():
        fail(f"manifest model {index} path is not a file: {path}")
    if resolved_path.stat().st_size != model["size_bytes"]:
        fail(f"manifest model {index} size does not match the cached file")
    if sha256(resolved_path) != model["sha256"]:
        fail(f"manifest model {index} SHA-256 does not match the cached file")


def validate_manifest(manifest: dict, cache_root: Path) -> None:
    models = manifest.get("models")
    if not isinstance(models, list):
        fail("manifest models must be a list")
    if len(models) != EXPECTED_ARTIFACTS:
        fail(f"manifest models: expected {EXPECTED_ARTIFACTS}, got {len(models)}")
    if manifest.get("artifact_count") != EXPECTED_ARTIFACTS:
        fail(f"artifact_count: expected {EXPECTED_ARTIFACTS}, got {manifest.get('artifact_count')}")
    if manifest.get("catalog_count") != EXPECTED_CATALOG_IDS:
        fail(f"catalog_count: expected {EXPECTED_CATALOG_IDS}, got {manifest.get('catalog_count')}")

    resolved_root = cache_root.resolve()
    for index, model in enumerate(models):
        validate_manifest_row(model, index, resolved_root)

    by_backend = {
        backend: [model for model in models if model.get("backend") == backend] for backend in ("paddle-ocr", "sceptre")
    }
    expected = {
        "paddle-ocr": (EXPECTED_PADDLE_ARTIFACTS, EXPECTED_PADDLE_CATALOG_IDS),
        "sceptre": (EXPECTED_SCEPTRE_ARTIFACTS, EXPECTED_SCEPTRE_CATALOG_IDS),
    }
    for backend, (artifact_count, catalog_count) in expected.items():
        entries = by_backend[backend]
        catalogs = {entry.get("catalog_id") for entry in entries}
        if len(entries) != artifact_count:
            fail(f"{backend} artifacts: expected {artifact_count}, got {len(entries)}")
        if len(catalogs) != catalog_count or None in catalogs:
            fail(f"{backend} catalog IDs: expected {catalog_count}, got {len(catalogs)}")

    paddle_catalog_roles = {}
    for entry in by_backend["paddle-ocr"]:
        catalog_id = entry.get("catalog_id")
        role = entry.get("role")
        if catalog_id in paddle_catalog_roles and paddle_catalog_roles[catalog_id] != role:
            fail(f"paddle-ocr catalog role changed within {catalog_id}")
        paddle_catalog_roles[catalog_id] = role
    role_counts = {
        role: sum(1 for actual in paddle_catalog_roles.values() if actual == role)
        for role in ("detector", "classifier", "recognizer")
    }
    expected_roles = {"detector": 5, "classifier": 2, "recognizer": 15}
    if role_counts != expected_roles:
        fail(f"paddle-ocr role census: expected {expected_roles}, got {role_counts}")

    sceptre_roles = [entry.get("role") for entry in by_backend["sceptre"]]
    if sceptre_roles.count("detector") != 1 or sceptre_roles.count("recognizer") != 8:
        fail("sceptre role census: expected 1 detector and 8 recognizers")

    en_mobile = [entry for entry in by_backend["paddle-ocr"] if entry["catalog_id"] == "paddle-v2-rec-en_mobile"]
    expected_en_mobile = {
        (
            "v2/rec/en_mobile/model.onnx",
            "model",
            EN_MOBILE_MODEL_SHA256,
        ),
        (
            "v2/rec/en_mobile/dict.txt",
            "dictionary",
            EN_MOBILE_DICTIONARY_SHA256,
        ),
    }
    actual_en_mobile = {(entry["file"], entry["artifact_kind"], entry["sha256"]) for entry in en_mobile}
    if actual_en_mobile != expected_en_mobile:
        fail("paddle-v2-rec-en_mobile coordinates or SHA-256 digests changed")


def run(
    command: list[str], *, env: dict[str, str], timeout: int, expect_success: bool | None = True
) -> subprocess.CompletedProcess:
    result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=timeout, check=False)
    if expect_success is True and result.returncode != 0:
        fail(f"command failed ({result.returncode}): {' '.join(command)}\n{result.stderr[-4000:]}")
    if expect_success is False and result.returncode == 0:
        fail(f"negative control unexpectedly succeeded: {' '.join(command)}")
    return result


def seed(binary: Path, cache_root: Path, env: dict[str, str], timeout: int) -> tuple[bytes, dict]:
    result = run(
        [str(binary), "cache", "seed-classic-ocr", "--cache-dir", str(cache_root), "--format", "json"],
        env=env,
        timeout=timeout,
    )
    encoded = result.stdout.encode()
    try:
        manifest = json.loads(encoded)
    except json.JSONDecodeError as error:
        fail(f"seed command did not emit JSON: {error}")
    validate_manifest(manifest, cache_root)
    if Path(manifest.get("cache_dir", "")).resolve() != cache_root:
        fail(f"manifest cache_dir does not match requested root: {manifest.get('cache_dir')}")
    return encoded, manifest


def extraction_command(binary: Path, fixture: Path, backend: str, job: OcrJob | PaddleJob) -> list[str]:
    config: dict = {}
    if backend == "paddle-ocr":
        config = {
            "ocr": {
                "paddle_ocr_settings": {
                    "model_version": job.model_version,
                    "model_tier": job.model_tier,
                    "use_angle_cls": job.use_angle_cls,
                }
            }
        }
    command = [
        str(binary),
        "extract",
        str(fixture),
        "--no-config-discovery",
        "--format",
        "json",
        "--config-json",
        json.dumps(config, separators=(",", ":")),
        "--ocr",
        "true",
        "--force-ocr",
        "true",
        "--ocr-backend",
        backend,
        "--ocr-language",
        job.language,
        "--no-cache",
        "true",
    ]
    if backend == "paddle-ocr" and job.auto_rotate:
        command.extend(["--ocr-auto-rotate", "true"])
    return command


def run_offline_matrix(binary: Path, fixture_root: Path, env: dict[str, str], timeout: int) -> int:
    jobs = tuple(("paddle-ocr", job) for job in PADDLE_JOBS) + tuple(("sceptre", job) for job in SCEPTRE_JOBS)
    expected_jobs = 23
    if len(jobs) != expected_jobs:
        fail(f"offline job census: expected {expected_jobs}, got {len(jobs)}")
    completed = 0
    for backend, job in jobs:
        fixture = fixture_root / job.fixture
        if not fixture.is_file():
            fail(f"required fixture missing: {fixture}")
        result = run(extraction_command(binary, fixture, backend, job), env=env, timeout=timeout)
        try:
            envelope = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            fail(f"{backend}/{job.name} did not emit JSON: {error}")
        if not isinstance(envelope.get("result"), dict):
            fail(f"{backend}/{job.name} JSON is missing the result document")
        completed += 1
    if completed != expected_jobs:
        fail(f"offline commands completed: expected {expected_jobs}, got {completed}")
    return completed


def prove_missing_cache_fails_for_both_backends(
    binary: Path, cache_root: Path, fixture_root: Path, env: dict[str, str], timeout: int
) -> int:
    for variable in ("HF_HUB_OFFLINE", "HUGGINGFACE_HUB_OFFLINE"):
        if env.get(variable) != "1":
            fail(f"missing-cache controls require {variable}=1")
    expected_controls = 2
    if len(MISSING_CACHE_JOBS) != expected_controls:
        fail(f"missing-cache control census: expected {expected_controls}, got {len(MISSING_CACHE_JOBS)}")
    hidden = cache_root.with_name(f"{cache_root.name}-hidden")
    if hidden.exists():
        fail(f"negative-control destination already exists: {hidden}")
    cache_root.rename(hidden)
    cache_root.mkdir()
    completed = []
    try:
        missing_env = dict(env)
        missing_env["XBERG_OCR_MODEL_CACHE_DIR"] = str(cache_root)
        for backend, job in MISSING_CACHE_JOBS:
            result = run(
                extraction_command(binary, fixture_root / job.fixture, backend, job),
                env=missing_env,
                timeout=timeout,
                expect_success=False,
            )
            diagnostic = (result.stdout + result.stderr).lower()
            if "offline" not in diagnostic or not any(term in diagnostic for term in ("model", "cache")):
                fail(f"{backend} missing-cache negative control failed for an unrelated reason")
            completed.append(backend)
    finally:
        shutil.rmtree(cache_root)
        hidden.rename(cache_root)
    if completed != ["paddle-ocr", "sceptre"]:
        fail(f"missing-cache controls completed in an unexpected census: {completed}")
    return len(completed)


def prove_non_ocr_cache_isolation(
    binary: Path, cache_root: Path, work_root: Path, env: dict[str, str], timeout: int
) -> None:
    hf_cache = work_root / "non-ocr-hf"
    before = snapshot_tree(cache_root)
    probe_env = dict(env)
    probe_env["HF_HUB_CACHE"] = str(hf_cache)
    probe_env["XBERG_OCR_MODEL_CACHE_DIR"] = str(cache_root)
    result = run(
        [
            str(binary),
            "embed",
            "--text",
            "cache-isolation-probe",
            "--preset",
            "fast",
            "--provider",
            "local",
            "--format",
            "json",
        ],
        env=probe_env,
        timeout=timeout,
    )
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        fail(f"embedding isolation probe did not emit JSON: {error}")
    if not isinstance(report, dict) or set(report) != {"model", "count", "dimensions", "embeddings"}:
        fail("embedding isolation probe JSON shape changed")
    embeddings = report["embeddings"]
    if (
        report["model"] != "fast"
        or type(report["count"]) is not int
        or report["count"] != 1
        or type(report["dimensions"]) is not int
        or report["dimensions"] != 384
        or not isinstance(embeddings, list)
        or len(embeddings) != 1
        or not isinstance(embeddings[0], list)
        or len(embeddings[0]) != 384
        or any(not isinstance(value, (int, float)) or isinstance(value, bool) for value in embeddings[0])
    ):
        fail("embedding isolation probe did not return one 384-dimensional fast vector")

    snapshot_tree(hf_cache)
    model = (
        hf_cache
        / "models--xberg-io--embedding-models"
        / "snapshots"
        / FAST_EMBEDDING_REVISION
        / FAST_EMBEDDING_MODEL_FILE
    )
    if not model.is_file():
        fail(f"fast embedding model is absent from its pinned cache path: {model}")
    if sha256(model) != FAST_EMBEDDING_MODEL_SHA256:
        fail("fast embedding model SHA-256 does not match the repository pin")
    assert_tree_unchanged(before, snapshot_tree(cache_root))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--xberg-bin", type=Path, default=Path("target/release/xberg"))
    parser.add_argument("--cache-dir")
    parser.add_argument("--fixture-root", type=Path, default=Path("test_documents"))
    parser.add_argument("--timeout-seconds", type=int, default=900)
    parser.add_argument("--keep-cache", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    binary = args.xberg_bin.resolve()
    if not binary.is_file():
        fail(f"xberg binary not found: {binary}; run `task rust:cli:build` first")
    fixture_root = args.fixture_root.resolve()
    if not fixture_root.is_dir():
        fail(f"test_documents not found: {fixture_root}")

    owned_temp = args.cache_dir is None
    raw_cache = args.cache_dir
    cache_root = validate_cache_root(
        Path(raw_cache) if raw_cache is not None else Path(tempfile.mkdtemp(prefix="xberg-classic-ocr-cache-")),
        raw_value=raw_cache,
    )
    cache_root.mkdir(parents=True, exist_ok=True)
    if any(cache_root.iterdir()):
        fail(f"cache root must start empty: {cache_root}")
    work_root = cache_root.parent / f"{cache_root.name}-acceptance-work"
    work_root.mkdir()
    try:
        env = dict(os.environ)
        env["XBERG_CACHE_DIR"] = str(work_root / "xberg-cache")
        env["XBERG_OCR_MODEL_CACHE_DIR"] = str(cache_root)
        env.pop("HF_HUB_OFFLINE", None)
        env.pop("HUGGINGFACE_HUB_OFFLINE", None)

        first_bytes, _ = seed(binary, cache_root, env, args.timeout_seconds)
        first_tree = snapshot_tree(cache_root)
        second_bytes, _ = seed(binary, cache_root, env, args.timeout_seconds)
        second_tree = snapshot_tree(cache_root)
        if first_bytes != second_bytes:
            fail("seed manifests were not byte-identical")
        assert_tree_unchanged(first_tree, second_tree)
        prove_non_ocr_cache_isolation(binary, cache_root, work_root, env, args.timeout_seconds)

        offline_env = dict(env)
        offline_env["HF_HUB_OFFLINE"] = "1"
        offline_env["HUGGINGFACE_HUB_OFFLINE"] = "1"
        completed = run_offline_matrix(binary, fixture_root, offline_env, args.timeout_seconds)
        negative_controls = prove_missing_cache_fails_for_both_backends(
            binary, cache_root, fixture_root, offline_env, args.timeout_seconds
        )
        print(
            json.dumps(
                {
                    "artifact_count": EXPECTED_ARTIFACTS,
                    "catalog_count": EXPECTED_CATALOG_IDS,
                    "offline_extraction_jobs": completed,
                    "missing_cache_negative_controls": negative_controls,
                    "cache_dir": str(cache_root),
                },
                sort_keys=True,
            )
        )
        return 0
    finally:
        shutil.rmtree(work_root, ignore_errors=True)
        if owned_temp and not args.keep_cache:
            shutil.rmtree(cache_root, ignore_errors=True)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (AcceptanceError, subprocess.TimeoutExpired) as error:
        print(f"classic OCR cache acceptance failed: {error}", file=sys.stderr)
        sys.exit(1)
