from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
VALIDATOR_PATH = Path(__file__).with_name("validate-workflow-matrix.py")
SPEC = importlib.util.spec_from_file_location("validate_workflow_matrix", VALIDATOR_PATH)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)
WORKFLOW = (REPOSITORY_ROOT / ".github/workflows/benchmarks.yaml").read_text()


class TestWorkflowMatrixValidation(unittest.TestCase):
    def test_parses_a_flow_sequence_starting_after_the_dimension_key(self) -> None:
        lines = [
            "        cohort:",
            "          [",
            "            native-pdf-fast-b8,",
            "            native-office-fast,",
            "          ]",
            "    steps:",
        ]

        assert VALIDATOR.parse_list(lines, 0, "") == (["native-pdf-fast-b8", "native-office-fast"], 4)

    def test_rejects_a_dimension_with_no_sequence_before_the_next_field(self) -> None:
        lines = ["      matrix:", "        cohort:", "        mode: [single-file]", "    steps:"]

        with self.assertRaisesRegex(ValueError, "matrix dimension 'cohort' has no values"):
            VALIDATOR.parse_matrix("empty-cohort", lines)

    def test_expands_every_exact_contract_cell_once(self) -> None:
        cells = VALIDATOR.workflow_cells(WORKFLOW)

        assert len(cells) == 140
        assert len({VALIDATOR.cell_key(cell) for cell in cells}) == 140
        assert sum(not cell["optional"] for cell in cells) == 84
        pdfium_cells = [cell for cell in cells if cell["framework"].endswith("-baseline-pdfium")]
        assert len(pdfium_cells) == 4
        assert all(cell["cohort"] == "native-pdf-fast-b8" for cell in pdfium_cells)
        assert all(not cell["optional"] for cell in pdfium_cells)

    def test_smoke_profile_expands_exactly_the_smoke_cells(self) -> None:
        cells = VALIDATOR.workflow_cells(WORKFLOW, "smoke")

        # Mirrors bench_matrix::SMOKE_ARTIFACTS; CI checks the two against each other through
        # `validate-workflow-matrix.py --profile smoke` with the built harness.
        assert sorted(cell["artifact"] for cell in cells) == [
            "benchmarks-docling-markdown-single-file-native-pdf-fast-b8",
            "benchmarks-liteparse-markdown-single-file-native-pdf-fast-b8",
            "benchmarks-rust-baseline-markdown-batch-native-pdf-fast-b8",
            "benchmarks-rust-baseline-markdown-single-file-native-pdf-fast-b8",
            "benchmarks-rust-baseline-markdown-single-file-ocr-images-fast",
            "benchmarks-rust-baseline-markdown-single-file-ocr-pdf-fast-b4",
        ]
        assert not any(cell["optional"] for cell in cells)

    def test_smoke_excludes_do_not_change_the_full_expansion(self) -> None:
        stripped = "\n".join(line for line in WORKFLOW.splitlines() if "{ profile: smoke," not in line)

        full = {VALIDATOR.cell_key(cell) for cell in VALIDATOR.workflow_cells(WORKFLOW)}
        assert full == {VALIDATOR.cell_key(cell) for cell in VALIDATOR.workflow_cells(stripped)}
        assert len(VALIDATOR.workflow_cells(stripped, "smoke")) > len(VALIDATOR.workflow_cells(WORKFLOW, "smoke"))

    def test_a_job_without_a_profile_dimension_must_skip_smoke_runs(self) -> None:
        mutated = WORKFLOW.replace(
            "github.ref == 'refs/heads/main' && inputs.profile != 'smoke' && (needs.setup.result == 'success' && !cancelled())",
            "github.ref == 'refs/heads/main' && (needs.setup.result == 'success' && !cancelled())",
            1,
        )
        assert mutated != WORKFLOW

        with self.assertRaisesRegex(ValueError, "must skip smoke runs"):
            VALIDATOR.workflow_cells(mutated, "smoke")

    def test_profile_dimension_must_follow_the_run_input(self) -> None:
        mutated = WORKFLOW.replace("""profile: ["${{ inputs.profile || 'full' }}"]""", 'profile: ["smoke"]', 1)
        assert mutated != WORKFLOW

        with self.assertRaisesRegex(ValueError, "matrix profile must be"):
            VALIDATOR.workflow_cells(mutated)

    def test_framework_env_drift_changes_cell_even_when_artifact_name_is_unchanged(self) -> None:
        mutated = WORKFLOW.replace("FRAMEWORK: docling", "FRAMEWORK: docling-wrong", 1)
        original_cells = VALIDATOR.workflow_cells(WORKFLOW)
        mutated_cells = VALIDATOR.workflow_cells(mutated)
        original = next(cell for cell in original_cells if cell["framework"] == "docling")
        changed = next(cell for cell in mutated_cells if cell["artifact"] == original["artifact"])

        assert changed["artifact"] == original["artifact"]
        assert changed["framework"] == "docling-wrong"
        assert VALIDATOR.cell_key(changed) != VALIDATOR.cell_key(original)

    def test_benchmark_steps_cannot_suppress_failures(self) -> None:
        mutated = WORKFLOW.replace(
            "      - name: Run benchmark\n        env:",
            "      - name: Run benchmark\n        continue-on-error: true\n        env:",
            1,
        )

        with self.assertRaisesRegex(ValueError, "must not use continue-on-error"):
            VALIDATOR.workflow_cells(mutated)

    def test_artifacts_must_be_validated_before_consolidation(self) -> None:
        mutated = (
            WORKFLOW.replace(
                "      - name: Validate benchmark artifacts (all cohorts)",
                "      - name: TEMP validation gate",
                1,
            )
            .replace(
                "      - name: Consolidate results (all cohorts)",
                "      - name: Validate benchmark artifacts (all cohorts)",
                1,
            )
            .replace(
                "      - name: TEMP validation gate",
                "      - name: Consolidate results (all cohorts)",
                1,
            )
        )

        with self.assertRaisesRegex(ValueError, "validation gates must precede publication"):
            VALIDATOR.workflow_cells(mutated)

    def test_tika_jar_is_downloaded_once_and_shared(self) -> None:
        jobs = VALIDATOR.split_jobs(WORKFLOW)

        VALIDATOR.validate_tika_artifact(jobs)

    def test_tika_matrix_cannot_download_from_maven(self) -> None:
        mutated = WORKFLOW.replace(
            "      - name: Download Apache Tika artifact\n        uses: actions/download-artifact@v8",
            "      - name: Download Apache Tika artifact\n        run: curl https://repo1.maven.org/tika.jar",
            1,
        )

        with self.assertRaisesRegex(ValueError, "must not download Apache Tika independently"):
            VALIDATOR.workflow_cells(mutated)

    def test_tika_download_must_verify_pinned_checksum(self) -> None:
        mutated = WORKFLOW.replace("${TIKA_SHA256}  ${jar_path}", "unchecked ${jar_path}", 1)

        with self.assertRaisesRegex(ValueError, "verify the pinned Apache Tika checksum"):
            VALIDATOR.workflow_cells(mutated)

    def test_tika_maven_download_must_be_unique(self) -> None:
        url = "repo.maven.apache.org/maven2/org/apache/tika/"
        mutated = WORKFLOW.replace(url, f"{url}{url}", 1)

        with self.assertRaisesRegex(ValueError, "download Apache Tika exactly once"):
            VALIDATOR.workflow_cells(mutated)


if __name__ == "__main__":
    unittest.main()
