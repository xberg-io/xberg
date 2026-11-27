import importlib.util
import sys
import time
import unittest
from pathlib import Path
from types import ModuleType

# ruff: noqa: D101, D102, PT009


def _load_module() -> ModuleType:
    script = Path(__file__).parents[1] / "scripts" / "build_corpus.py"
    spec = importlib.util.spec_from_file_location("build_corpus_under_test", script)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    # @dataclass resolves string annotations through sys.modules[cls.__module__].
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


strip_to_text = _load_module().strip_to_text


class StripToTextMathTests(unittest.TestCase):
    def test_currency_dollar_is_kept_as_prose(self):
        md = "The fee rose from $160 to $175 per month.\n\nInline $x^2$ is math.\n"

        self.assertEqual(strip_to_text(md), "The fee rose from $160 to $175 per month.\nInline is math.\n")

    def test_inline_math_closing_before_a_digit_is_removed(self):
        md = "Roughly $\\sim$240 million years ago.\n"

        self.assertEqual(strip_to_text(md), "Roughly 240 million years ago.\n")

    def test_adjacent_inline_spans_are_two_spans(self):
        md = "Let $a$$b$ denote the pair and keep this prose.\n"

        self.assertEqual(strip_to_text(md), "Let denote the pair and keep this prose.\n")

    def test_display_math_is_removed(self):
        md = "Before the equation\n$$ x = \\frac{a}{b} $$\nafter the equation.\n"

        self.assertEqual(strip_to_text(md), "Before the equation\nafter the equation.\n")

    def test_multiline_display_math_is_removed(self):
        md = "Before\n$$\n\\begin{aligned}\na &= b \\\\\nc &= d\n\\end{aligned}\n$$\nAfter\n"

        self.assertEqual(strip_to_text(md), "Before\nAfter\n")

    def test_escaped_dollars_are_not_math(self):
        md = "It costs \\$5 or \\$6 depending on size.\n"

        self.assertEqual(strip_to_text(md), "It costs \\$5 or \\$6 depending on size.\n")

    def test_escaped_dollar_inside_inline_math_does_not_close_it(self):
        md = "Price $p = \\$3$ applies here.\n"

        self.assertEqual(strip_to_text(md), "Price applies here.\n")

    def test_inline_math_does_not_span_a_blank_line(self):
        md = "First paragraph costs $5.\n\nSecond paragraph keeps its text$ too.\n"

        self.assertEqual(strip_to_text(md), "First paragraph costs $5.\nSecond paragraph keeps its text$ too.\n")

    def test_display_math_does_not_span_a_blank_line(self):
        md = "Stray $$ opener here.\n\nThis paragraph survives.\n\nLater $$ y $$ closes.\n"

        self.assertEqual(
            strip_to_text(md),
            "Stray $$ opener here.\nThis paragraph survives.\nLater closes.\n",
        )

    def test_inline_math_spans_a_single_line_break(self):
        md = "Wrapped $a +\nb$ math.\n"

        self.assertEqual(strip_to_text(md), "Wrapped math.\n")

    def test_adversarial_dollar_runs_finish_fast(self):
        inputs = [
            "$" * 200_000,
            "$a " * 100_000,
            "$$ " + "x " * 200_000,
            ("$ a" + "\n" * 2) * 50_000,
            "\\$" * 100_000,
            "$x" + "\\" * 200_000,
        ]
        start = time.perf_counter()
        for md in inputs:
            strip_to_text(md)

        self.assertLess(time.perf_counter() - start, 5.0)


if __name__ == "__main__":
    unittest.main()
