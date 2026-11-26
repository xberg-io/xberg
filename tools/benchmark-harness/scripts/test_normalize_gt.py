import shutil
import sys
import unittest
from pathlib import Path

# ~keep: This script intentionally uses unittest so it can run without pytest.
# ruff: noqa: D101, D102, PT009

sys.path.insert(0, str(Path(__file__).resolve().parent))
from normalize_gt import normalize_with_report


@unittest.skipUnless(shutil.which("pandoc"), "pandoc is required for the LaTeX table transform")
class LatexTableTransformTests(unittest.TestCase):
    def test_should_convert_a_raw_latex_table_to_a_gfm_pipe_table(self):
        md = "Before.\n\\begin{table}\n\\begin{tabular}{c c}\n\\hline\nA & B \\\\\n\\hline\n1 & $x$ \\\\\n\\end{tabular}\n\\end{table}\nTable 1: Caption.\n"

        out, report = normalize_with_report(md, source="readoc")

        self.assertEqual(report.get("latex_table"), 1)
        self.assertNotIn("\\hline", out)
        self.assertNotIn("\\begin", out)
        self.assertRegex(out, r"\|\s*A\s*\|\s*B\s*\|")
        self.assertIn("$x$", out)
        self.assertNotIn("$`", out)
        self.assertIn("\n\nTable 1: Caption.", out)

    def test_should_keep_literal_bold_markers_inside_converted_cells(self):
        md = "\\begin{tabular}{c c}\n**Metric** & **FS** \\\\\n1 & 2 \\\\\n\\end{tabular}\n"

        out, _ = normalize_with_report(md, source="readoc")

        self.assertRegex(out, r"\|\s*\*\*Metric\*\*\s*\|")
        self.assertNotIn("\\*\\*", out)

    def test_should_leave_non_readoc_sources_untouched(self):
        md = "\\begin{tabular}{c}\nA \\\\\n\\end{tabular}\n"

        out, report = normalize_with_report(md, source="federal_register")

        self.assertNotIn("latex_table", report)
        self.assertIn("\\begin{tabular}", out)


if __name__ == "__main__":
    unittest.main()
