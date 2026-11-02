"""Acquisition-to-stage controls use synthetic documents and untouched raw HTML."""

import copy
import hashlib
import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# ruff: noqa: PT027

HARNESS = Path(__file__).parents[1]
CORPUS = Path(os.environ.get("XBERG_BENCHMARK_CORPUS_ROOT", HARNESS.parents[1] / "test_documents"))
sys.path.insert(0, str(HARNESS / "parsebench_html"))


def _stage_module():
    spec = importlib.util.spec_from_file_location("parsebench_staging", HARNESS / "parsebench_html" / "stage.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _digest(payload):
    return hashlib.sha256(payload).hexdigest()


class StagingTest(unittest.TestCase):
    """Pin validation must fire before any qualification stage is accepted."""

    def setUp(self):
        """Build a complete synthetic 503-record inventory through the real child schema."""
        self.stage = _stage_module()
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cache = self.root / "cache"
        self.output = self.root / "stage"
        self.registry = self.root / "registry.json"
        descriptor = json.loads((CORPUS / "scripts/data/benchmark-upstreams.json").read_text())
        self.objects = {
            key: value for key, value in descriptor["objects"].items() if value["source"] != "parsebench-tables"
        }
        self.rows = []
        for index in range(503):
            row_id = f"synthetic-{index}"
            upstream = f"docs/table/synthetic-{index}.pdf"
            row = {
                "id": row_id,
                "pdf": upstream,
                "category": "table",
                "type": "expected_markdown",
                "expected_markdown": " <table>\r\n<tr><td>é &amp; 1</td></tr></table>\n ",
                "rule": '{"trm_unsupported": true}',
                "tags": ["synthetic"],
                "page": None,
            }
            self.rows.append(row)
            self._pin(upstream, b"%PDF-synthetic\n" + str(index).encode(), "pdf", _digest(row_id.encode())[:20])
        self.jsonl = ("\n".join(json.dumps(row, ensure_ascii=False) for row in self.rows) + "\n").encode()
        self._pin("table.jsonl", self.jsonl, "jsonl", None)
        descriptor["objects"] = self.objects
        self.descriptor = descriptor
        self.registry.write_text(json.dumps(descriptor))
        self.checksum = patch.object(self.stage, "JSONL_SHA256", _digest(self.jsonl))
        self.checksum.start()
        self.addCleanup(self.checksum.stop)

    def _pin(self, upstream, payload, kind, record_id):
        key = f"parsebench-tables/{upstream}"
        path = self.cache / key
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
        revision = "2805a1d940f95a203e0ae4b88be9934f7765b3fc"
        self.objects[key] = {
            "sha256": _digest(payload),
            "size": len(payload),
            "source": "parsebench-tables",
            "upstream_path": upstream,
            "url": f"https://huggingface.co/datasets/llamaindex/ParseBench/resolve/{revision}/{upstream}",
            "kind": kind,
            "group": "table",
            "id": record_id,
            "content_rights": "reference/unreviewed",
        }

    def _build(self, **kwargs):
        return self.stage.build_stage(CORPUS, self.cache, self.output, registry=self.registry, **kwargs)

    def _rewrite_rows(self, rows):
        payload = ("\n".join(json.dumps(row) for row in rows) + "\n").encode()
        self._pin("table.jsonl", payload, "jsonl", None)
        self.descriptor["objects"] = self.objects
        self.registry.write_text(json.dumps(self.descriptor))
        self.stage.JSONL_SHA256 = _digest(payload)

    def test_all_records_raw_oracles_and_rules_survive(self):
        """All records raw oracles and rules survive."""
        value = self._build()
        assert len(value["records"]) == 503
        assert len({row["id"] for row in value["records"]}) == 503
        assert value["records"] == sorted(value["records"], key=lambda row: row["id"])
        for row in value["records"]:
            assert (self.output / row["oracle"]["file"]).read_bytes() == row["upstream"]["expected_markdown"].encode()
            assert row["upstream"]["rule"] == '{"trm_unsupported": true}'
            assert row["curation"]["gate_verdict"] == "UNREVIEWED"
        assert value["annotations_license_url"] == "https://www.apache.org/licenses/LICENSE-2.0"

    def test_wrong_pdf_hash_rejected(self):
        """Wrong pdf hash rejected."""
        (self.cache / "parsebench-tables/docs/table/synthetic-0.pdf").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "pin|mismatch"):
            self._build()
        assert not (self.output / "stage.json").exists()

    def test_missing_pdf_rejected(self):
        """Missing pdf rejected."""
        (self.cache / "parsebench-tables/docs/table/synthetic-0.pdf").unlink()
        with self.assertRaisesRegex(ValueError, "missing|pin"):
            self._build()

    def test_wrong_jsonl_hash_rejected(self):
        """Wrong jsonl hash rejected."""
        (self.cache / "parsebench-tables/table.jsonl").write_bytes(b"{}\n")
        with self.assertRaisesRegex(ValueError, "pin|mismatch"):
            self._build()

    def test_changed_jsonl_cannot_be_reauthorized_by_registry(self):
        """Changed jsonl cannot be reauthorized by registry."""
        self._rewrite_rows(self.rows)
        self.stage.JSONL_SHA256 = "0" * 64
        with self.assertRaisesRegex(ValueError, "pinned.*JSONL"):
            self._build()

    def test_wrong_revision_rejected(self):
        """Wrong revision rejected."""
        self.descriptor["sources"]["parsebench-tables"]["dataset_revision"] = "0" * 40
        self.registry.write_text(json.dumps(self.descriptor))
        with self.assertRaisesRegex(ValueError, "pinned|sources"):
            self._build()

    def test_helper_and_registry_cannot_relabel_updated_upstream_as_frozen_revision(self):
        """A future consistent child update must fail the adapter's own frozen source contract."""
        helper = self.stage._helpers(CORPUS)
        sources = copy.deepcopy(helper.SOURCES)
        previous = sources["parsebench-tables"]["dataset_revision"]
        sources["parsebench-tables"]["dataset_revision"] = "f" * 40
        sources["parsebench-tables"]["evaluator_revision"] = "e" * 40
        self.descriptor["sources"] = sources
        for pin in self.descriptor["objects"].values():
            if pin["source"] == "parsebench-tables":
                pin["url"] = pin["url"].replace(previous, "f" * 40)
        self.registry.write_text(json.dumps(self.descriptor))
        with patch.object(helper, "SOURCES", sources):
            assert len(helper.load_inventory(self.registry)) == 1493
            with self.assertRaisesRegex(ValueError, "adapter.*frozen|frozen.*source"):
                self._build()
        assert not (self.output / "stage.json").exists()

    def test_duplicate_id_rejected(self):
        """Duplicate id rejected."""
        self.rows[1]["id"] = self.rows[0]["id"]
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "duplicate|identity"):
            self._build()

    def test_duplicate_pdf_rejected(self):
        """Duplicate pdf rejected."""
        self.rows[1]["pdf"] = self.rows[0]["pdf"]
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "duplicate|identity"):
            self._build()

    def test_pdf_path_traversal_rejected(self):
        """Pdf path traversal rejected."""
        self.rows[0]["pdf"] = "../escape.pdf"
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "PDF|path|pin"):
            self._build()

    def test_zero_work_and_missing_row_rejected(self):
        """Zero work and missing row rejected."""
        for rows in ([], self.rows[:-1]):
            with self.subTest(count=len(rows)):
                self._rewrite_rows(rows)
                with self.assertRaisesRegex(ValueError, "503|count"):
                    self._build()

    def test_jsonl_identity_must_match_pdf_pin(self):
        """Jsonl identity must match pdf pin."""
        self.rows[0]["id"] = "different-source-id"
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "identity"):
            self._build()

    def test_oracle_must_contain_original_html_table(self):
        """Oracle must contain original html table."""
        self.rows[0]["expected_markdown"] = "not HTML"
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "HTML"):
            self._build()

    def test_output_and_cache_cannot_be_inside_git(self):
        """Output and cache cannot be inside git."""
        (self.root / ".git").mkdir()
        with self.assertRaisesRegex(ValueError, "outside.*Git"):
            self._build()

    def test_nonempty_output_is_never_overwritten(self):
        """Nonempty output is never overwritten."""
        self.output.mkdir()
        sentinel = self.output / "existing"
        sentinel.write_text("keep")
        with self.assertRaisesRegex(ValueError, "empty|exists"):
            self._build()
        assert sentinel.read_text() == "keep"

    def test_symlink_escape_rejected_without_network(self):
        """Symlink escape rejected without network."""
        path = self.cache / "parsebench-tables/docs/table/synthetic-0.pdf"
        external = self.root / "outside.pdf"
        external.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(external)
        with self.assertRaisesRegex(ValueError, "escapes"):
            self._build()

    def test_output_inside_git_rejected_with_external_valid_cache(self):
        """A valid external cache does not authorize an output inside Git."""
        checkout = self.root / "checkout"
        checkout.mkdir()
        (checkout / ".git").write_text("gitdir: elsewhere")
        self.output = checkout / "stage"
        with self.assertRaisesRegex(ValueError, "outside.*Git"):
            self._build()

    def test_optional_curation_is_reference_only_and_never_filters(self):
        """A rejected curation verdict retains all 503 source records."""
        curated = self.root / "curated.json"
        curated.write_text(
            json.dumps(
                {
                    "documents": [
                        {
                            "id": "pb_synthetic-0",
                            "source_dataset": "parsebench",
                            "source_revision": self.descriptor["sources"]["parsebench-tables"]["dataset_revision"],
                            "src_gt_sha256": _digest(self.rows[0]["expected_markdown"].encode()),
                            "gate_verdict": "REJECT",
                        }
                    ]
                }
            )
        )
        value = self._build(curated_manifest=curated)
        assert len(value["records"]) == 503
        assert sum(row["curation"]["gate_verdict"] == "REJECT" for row in value["records"]) == 1
        assert sum(row["curation"]["gate_verdict"] == "UNREVIEWED" for row in value["records"]) == 502

    def test_stale_curated_raw_source_hash_is_rejected(self):
        """Optional curation must refer to the exact raw oracle."""
        curated = self.root / "curated.json"
        curated.write_text(
            json.dumps(
                {
                    "documents": [
                        {
                            "id": "pb_synthetic-0",
                            "source_dataset": "parsebench",
                            "source_revision": self.descriptor["sources"]["parsebench-tables"]["dataset_revision"],
                            "src_gt_sha256": "0" * 64,
                            "gate_verdict": "ACCEPT",
                        }
                    ]
                }
            )
        )
        with self.assertRaisesRegex(ValueError, "curated.*pinned"):
            self._build(curated_manifest=curated)
        assert not (self.output / "stage.json").exists()

    def test_evaluation_rule_must_remain_an_object(self):
        """Malformed upstream evaluator options cannot create a usable stage."""
        self.rows[0]["rule"] = "[]"
        self._rewrite_rows(self.rows)
        with self.assertRaisesRegex(ValueError, "rule"):
            self._build()

    def test_pinned_jsonl_duplicate_object_keys_rejected(self):
        """JSON parsing cannot silently replace an ambiguous raw identity."""
        payload = self.jsonl.replace(b'{"id": "synthetic-0",', b'{"id": "ambiguous", "id": "synthetic-0",', 1)
        self._pin("table.jsonl", payload, "jsonl", None)
        self.registry.write_text(json.dumps(self.descriptor))
        self.stage.JSONL_SHA256 = _digest(payload)
        with self.assertRaisesRegex(ValueError, "duplicate JSON"):
            self._build()


if __name__ == "__main__":
    unittest.main()
