import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# ~keep: This script intentionally uses unittest so it can run without pytest.
# ruff: noqa: D101, D102, PT009, PT027

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_corpus as bc


class StripToTextTests(unittest.TestCase):
    def test_should_keep_the_text_of_fenced_code_and_drop_fences_and_info_string(self):
        md = "Intro.\n\n```python\n>>> import datetime\n# a comment | with pipe\n```\n\nAfter.\n"

        self.assertEqual(bc.strip_to_text(md), "Intro.\n>>> import datetime\n# a comment | with pipe\nAfter.\n")

    def test_should_keep_inline_code_text_separated_by_spaces(self):
        md = "An attacker with the`iam:CreatePolicyVersion`permission and a `validate()` method.\n"

        self.assertEqual(
            bc.strip_to_text(md), "An attacker with the iam:CreatePolicyVersion permission and a validate() method.\n"
        )

    def test_should_drop_gfm_math_blocks_and_spans(self):
        md = "Eq:\n\n``` math\nx^2\n```\n\nCell $`N_{r}`$ and $y$ end.\n"

        self.assertEqual(bc.strip_to_text(md), "Eq:\nCell and end.\n")

    def test_should_still_strip_markup_outside_code(self):
        md = "# Title\n\n| a | **b** |\n| --- | --- |\n\n> quote with $160 price\n"

        self.assertEqual(bc.strip_to_text(md), "Title\na b\n--- ---\nquote with $160 price\n")


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


class GtOverrideTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.gt_dir = root / "gt"
        self.gt_dir.mkdir()
        self.manifest = root / "corpus_manifest.json"
        self.md, self.txt = "# Corrected\n\nText on the page.\n", "Corrected\nText on the page.\n"
        (self.gt_dir / "doc1.md").write_text(self.md)
        (self.gt_dir / "doc1.txt").write_text(self.txt)
        self.doc = {
            "id": "doc1",
            "source_dataset": "federal_register",
            "gt_override": "manual",
            "out_gt_md_sha256": _sha(self.md),
            "gt_txt_sha256": _sha(self.txt),
        }
        self.manifest.write_text(json.dumps({"documents": [self.doc]}))
        self.patches = [
            patch.object(bc, "MANIFEST", self.manifest),
            patch.object(bc, "_dest", lambda _r: (root / "pdf", self.gt_dir, "prefix")),
        ]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.tmp.cleanup()

    def test_should_carry_the_override_mark_and_hashes_from_the_manifest_into_the_ledger(self):
        records = {"doc1": {"id": "doc1", "out_gt_md_sha256": "stale", "gt_txt_sha256": "stale"}}

        bc._merge_overrides(records)

        self.assertEqual(records["doc1"]["gt_override"], "manual")
        self.assertEqual(records["doc1"]["out_gt_md_sha256"], _sha(self.md))

    def test_should_not_overwrite_the_override_hash_with_the_regenerated_gt(self):
        rec = dict(self.doc)

        bc._record_generated_gt(rec, "# Upstream\n")

        self.assertEqual(rec["out_gt_md_sha256"], _sha(self.md))

    def test_should_read_the_published_override_files(self):
        self.assertEqual(bc._read_overrides([dict(self.doc)]), {"doc1": (self.md, self.txt)})

    def test_should_fail_when_the_published_override_no_longer_matches_the_manifest(self):
        (self.gt_dir / "doc1.md").write_text("# Regenerated upstream\n")

        with self.assertRaises(RuntimeError):
            bc._read_overrides([dict(self.doc)])


if __name__ == "__main__":
    unittest.main()
