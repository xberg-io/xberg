import sys
import unittest
from pathlib import Path

# ~keep: This script intentionally uses unittest so it can run without pytest.
# ruff: noqa: D101, D102, PT009

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


if __name__ == "__main__":
    unittest.main()
