import hashlib
import importlib.util
import os
from pathlib import Path
from types import ModuleType

import pytest

SCRIPT = Path(__file__).with_name("classic_ocr_cache_acceptance.py")

PADDLE_CATALOGS = (
    tuple((f"paddle-detector-{index}", "detector") for index in range(5))
    + tuple((f"paddle-classifier-{index}", "classifier") for index in range(2))
    + (("paddle-v2-rec-en_mobile", "recognizer"),)
    + tuple((f"paddle-recognizer-{index}", "recognizer") for index in range(14))
)


def load_harness() -> ModuleType:
    spec = importlib.util.spec_from_file_location("classic_ocr_cache_acceptance", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build_manifest(cache_root: Path) -> dict:
    models = []
    artifact_index = 0
    for catalog_id, role in PADDLE_CATALOGS:
        kinds = ("model", "dictionary") if role == "recognizer" else ("model",)
        for kind in kinds:
            payload = f"artifact-{artifact_index}".encode()
            path = cache_root / f"artifact-{artifact_index}.bin"
            path.write_bytes(payload)
            file = f"models/artifact-{artifact_index}.bin"
            if catalog_id == "paddle-v2-rec-en_mobile":
                file = f"v2/rec/en_mobile/{'model.onnx' if kind == 'model' else 'dict.txt'}"
            models.append(
                {
                    "backend": "paddle-ocr",
                    "catalog_id": catalog_id,
                    "role": role,
                    "artifact_kind": kind,
                    "repo": "xberg-io/paddleocr-onnx-models",
                    "revision": "a" * 40,
                    "file": file,
                    "sha256": hashlib.sha256(payload).hexdigest(),
                    "license": "Apache-2.0",
                    "size_bytes": len(payload),
                    "path": str(path),
                }
            )
            artifact_index += 1
    for catalog_index in range(9):
        payload = f"artifact-{artifact_index}".encode()
        path = cache_root / f"artifact-{artifact_index}.bin"
        path.write_bytes(payload)
        models.append(
            {
                "backend": "sceptre",
                "catalog_id": f"sceptre-{catalog_index}",
                "role": "detector" if catalog_index == 0 else "recognizer",
                "artifact_kind": "model",
                "repo": "JaidedAI/EasyOCR",
                "revision": "b" * 40,
                "file": f"models/artifact-{artifact_index}.bin",
                "sha256": hashlib.sha256(payload).hexdigest(),
                "license": "Apache-2.0",
                "size_bytes": len(payload),
                "path": str(path),
            }
        )
        artifact_index += 1
    return {"artifact_count": 46, "catalog_count": 31, "models": models}


def test_should_define_exact_backend_job_census() -> None:
    harness = load_harness()

    assert len(harness.PADDLE_JOBS) == 15
    assert len(harness.SCEPTRE_JOBS) == 8
    assert len(harness.PADDLE_JOBS) + len(harness.SCEPTRE_JOBS) == 23
    assert {job.name for job in harness.PADDLE_JOBS if job.model_version == "pp-ocrv5"} == {
        "english",
        "chinese",
        "latin",
        "korean",
        "eslav",
        "thai",
        "greek",
        "arabic",
        "devanagari",
        "tamil",
        "telugu",
        "english_server",
    }
    assert {(job.model_version, job.model_tier) for job in harness.PADDLE_JOBS if job.model_version == "pp-ocrv6"} == {
        ("pp-ocrv6", "medium"),
        ("pp-ocrv6", "small"),
        ("pp-ocrv6", "tiny"),
    }
    classifier_jobs = [job for job in harness.PADDLE_JOBS if job.use_angle_cls and job.auto_rotate]
    assert [(job.name, job.model_version, job.model_tier) for job in classifier_jobs] == [
        ("english", "pp-ocrv5", "mobile")
    ]
    assert {job.name for job in harness.SCEPTRE_JOBS} == {
        "english",
        "latin",
        "simplified_chinese",
        "japanese",
        "korean",
        "telugu",
        "kannada",
        "cyrillic",
    }


def test_should_define_both_missing_cache_negative_controls() -> None:
    harness = load_harness()

    assert [(backend, job.name) for backend, job in harness.MISSING_CACHE_JOBS] == [
        ("paddle-ocr", "english"),
        ("sceptre", "english"),
    ]


def test_should_run_both_missing_cache_controls_offline(tmp_path: Path) -> None:
    harness = load_harness()
    binary = tmp_path / "xberg-fake"
    log = tmp_path / "commands.log"
    binary.write_text(
        '#!/bin/sh\nprintf "%s\\n" "$*" >> "$ACCEPTANCE_LOG"\necho "offline model cache missing" >&2\nexit 2\n'
    )
    binary.chmod(0o755)
    cache_root = tmp_path / "ocr-cache"
    cache_root.mkdir()
    (cache_root / "seeded-model").write_bytes(b"model")
    fixture_root = tmp_path / "test_documents"
    (fixture_root / "images").mkdir(parents=True)
    (fixture_root / "images" / "english.png").write_bytes(b"fixture")
    env = dict(os.environ)
    env.update(
        {
            "ACCEPTANCE_LOG": str(log),
            "HF_HUB_OFFLINE": "1",
            "HUGGINGFACE_HUB_OFFLINE": "1",
        }
    )

    completed = harness.prove_missing_cache_fails_for_both_backends(binary, cache_root, fixture_root, env, timeout=10)

    commands = log.read_text().splitlines()
    assert completed == 2
    assert len(commands) == 2
    assert "--ocr-backend paddle-ocr" in commands[0]
    assert "--ocr-backend sceptre" in commands[1]
    assert (cache_root / "seeded-model").read_bytes() == b"model"
    assert not cache_root.with_name("ocr-cache-hidden").exists()


def test_should_reject_empty_cache_root() -> None:
    harness = load_harness()

    with pytest.raises(harness.AcceptanceError, match=r"^cache root must not be empty$"):
        harness.validate_cache_root(Path(), raw_value="")


def test_should_validate_exact_manifest_census(tmp_path: Path) -> None:
    harness = load_harness()
    manifest = build_manifest(tmp_path)
    en_mobile = [model for model in manifest["models"] if model["catalog_id"] == "paddle-v2-rec-en_mobile"]
    harness.EN_MOBILE_MODEL_SHA256 = next(model["sha256"] for model in en_mobile if model["artifact_kind"] == "model")
    harness.EN_MOBILE_DICTIONARY_SHA256 = next(
        model["sha256"] for model in en_mobile if model["artifact_kind"] == "dictionary"
    )

    harness.validate_manifest(manifest, tmp_path)


def test_should_pin_the_backward_compatibility_only_en_mobile_coordinates() -> None:
    harness = load_harness()

    assert harness.EN_MOBILE_MODEL_SHA256 == "70b2450eed39599af6b996c27a2f1a0ef30eeb49f9f66dd3e74f28f652befc89"
    assert harness.EN_MOBILE_DICTIONARY_SHA256 == "854c6bb3e5a9a8ceac81fa700927e86a8da0e9b329a2846c57fc686be9db93e5"


def test_should_pin_fast_embedding_cache_coordinates() -> None:
    harness = load_harness()

    assert harness.FAST_EMBEDDING_REVISION == "4b127809f88a5aa1569d1238032b5ff40e5879bc"
    assert harness.FAST_EMBEDDING_MODEL_FILE == "all-MiniLM-L6-v2/model_quantized.onnx"
    assert harness.FAST_EMBEDDING_MODEL_SHA256 == "afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1"


def test_should_fail_when_manifest_census_is_vacuous(tmp_path: Path) -> None:
    harness = load_harness()
    manifest = {"artifact_count": 46, "catalog_count": 31, "models": []}

    with pytest.raises(harness.AcceptanceError, match=r"^manifest models: expected 46, got 0$"):
        harness.validate_manifest(manifest, tmp_path)


def test_should_fail_when_paddle_role_census_is_wrong(tmp_path: Path) -> None:
    harness = load_harness()
    manifest = build_manifest(tmp_path)
    manifest["models"][0]["role"] = "classifier"

    expected = (
        "paddle-ocr role census: expected {'detector': 5, 'classifier': 2, 'recognizer': 15}, "
        "got {'detector': 4, 'classifier': 3, 'recognizer': 15}"
    )
    with pytest.raises(harness.AcceptanceError) as raised:
        harness.validate_manifest(manifest, tmp_path)
    assert str(raised.value) == expected


def test_should_detect_any_second_seed_tree_write(tmp_path: Path) -> None:
    harness = load_harness()
    model = tmp_path / "model.onnx"
    model.write_bytes(b"first")
    first = harness.snapshot_tree(tmp_path)
    model.write_bytes(b"second")
    second = harness.snapshot_tree(tmp_path)

    with pytest.raises(harness.AcceptanceError, match=r"^cache tree changed during the second seed$"):
        harness.assert_tree_unchanged(first, second)


def test_should_seed_fast_embedding_in_normal_hf_cache_without_touching_ocr_cache(tmp_path: Path) -> None:
    harness = load_harness()
    binary = tmp_path / "xberg-fake"
    log = tmp_path / "commands.log"
    payload = b"fast-embedding-model"
    binary.write_text(
        "#!/bin/sh\n"
        'printf "%s\\n" "$*" > "$ACCEPTANCE_LOG"\n'
        'model="$HF_HUB_CACHE/models--xberg-io--embedding-models/snapshots/'
        f'{harness.FAST_EMBEDDING_REVISION}/{harness.FAST_EMBEDDING_MODEL_FILE}"\n'
        'mkdir -p "$(dirname "$model")"\n'
        f"printf '{payload.decode()}' > \"$model\"\n"
        "python3 - <<'PY'\n"
        "import json\n"
        "print(json.dumps({'model': 'fast', 'count': 1, 'dimensions': 384, 'embeddings': [[0.25] * 384]}))\n"
        "PY\n"
    )
    binary.chmod(0o755)
    cache_root = tmp_path / "ocr-cache"
    cache_root.mkdir()
    (cache_root / "seeded-model").write_bytes(b"ocr-model")
    work_root = tmp_path / "work"
    work_root.mkdir()
    env = dict(os.environ)
    env["ACCEPTANCE_LOG"] = str(log)
    harness.FAST_EMBEDDING_MODEL_SHA256 = hashlib.sha256(payload).hexdigest()

    harness.prove_non_ocr_cache_isolation(binary, cache_root, work_root, env, timeout=10)

    assert log.read_text().strip() == (
        "embed --text cache-isolation-probe --preset fast --provider local --format json"
    )
    assert (cache_root / "seeded-model").read_bytes() == b"ocr-model"
    model = (
        work_root
        / "non-ocr-hf"
        / "models--xberg-io--embedding-models"
        / "snapshots"
        / harness.FAST_EMBEDDING_REVISION
        / harness.FAST_EMBEDDING_MODEL_FILE
    )
    assert model.read_bytes() == payload


def test_should_reject_fast_embedding_cached_at_wrong_path(tmp_path: Path) -> None:
    harness = load_harness()
    binary = tmp_path / "xberg-fake"
    binary.write_text(
        "#!/bin/sh\n"
        'mkdir -p "$HF_HUB_CACHE/models--xberg-io--embedding-models/snapshots/wrong-revision"\n'
        'printf model > "$HF_HUB_CACHE/models--xberg-io--embedding-models/snapshots/wrong-revision/model.onnx"\n'
        "python3 - <<'PY'\n"
        "import json\n"
        "print(json.dumps({'model': 'fast', 'count': 1, 'dimensions': 384, 'embeddings': [[0.25] * 384]}))\n"
        "PY\n"
    )
    binary.chmod(0o755)
    cache_root = tmp_path / "ocr-cache"
    cache_root.mkdir()
    (cache_root / "seeded-model").write_bytes(b"ocr-model")
    work_root = tmp_path / "work"
    work_root.mkdir()

    with pytest.raises(harness.AcceptanceError, match=r"^fast embedding model is absent from its pinned cache path:"):
        harness.prove_non_ocr_cache_isolation(binary, cache_root, work_root, dict(os.environ), timeout=10)


def test_should_reject_embedding_probe_that_writes_to_ocr_cache(tmp_path: Path) -> None:
    harness = load_harness()
    binary = tmp_path / "xberg-fake"
    payload = b"fast-embedding-model"
    binary.write_text(
        "#!/bin/sh\n"
        'model="$HF_HUB_CACHE/models--xberg-io--embedding-models/snapshots/'
        f'{harness.FAST_EMBEDDING_REVISION}/{harness.FAST_EMBEDDING_MODEL_FILE}"\n'
        'mkdir -p "$(dirname "$model")"\n'
        f"printf '{payload.decode()}' > \"$model\"\n"
        'printf leaked > "$XBERG_OCR_MODEL_CACHE_DIR/non-ocr-leak"\n'
        "python3 - <<'PY'\n"
        "import json\n"
        "print(json.dumps({'model': 'fast', 'count': 1, 'dimensions': 384, 'embeddings': [[0.25] * 384]}))\n"
        "PY\n"
    )
    binary.chmod(0o755)
    cache_root = tmp_path / "ocr-cache"
    cache_root.mkdir()
    (cache_root / "seeded-model").write_bytes(b"ocr-model")
    work_root = tmp_path / "work"
    work_root.mkdir()
    harness.FAST_EMBEDDING_MODEL_SHA256 = hashlib.sha256(payload).hexdigest()

    with pytest.raises(harness.AcceptanceError, match=r"^cache tree changed during the second seed$"):
        harness.prove_non_ocr_cache_isolation(binary, cache_root, work_root, dict(os.environ), timeout=10)
