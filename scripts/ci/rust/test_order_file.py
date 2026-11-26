#!/usr/bin/env python3
"""Tests for ``scripts/ci/rust/order_file.py``: profile parsing, ordering and link-map checks.

The profile and map excerpts are verbatim output of rustc 1.100's ``llvm-profdata show``, ld64's
``-map`` and rust-lld's ``-Map``, trimmed to a few rows.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import order_file

PROFDATA_SHOW = """\
Counters:
  m.7f52311871bfba1c-cgu.0;_RNvCsaVJ4G3CYWvg_1m3hot:
    Hash: 0x0550aa894fffffff
    Counters: 2
    Block counts: [1000, 333]
  m.7f52311871bfba1c-cgu.0;_RNvCsaVJ4G3CYWvg_1m4cold:
    Hash: 0x0a4d0ad3efffffff
    Counters: 1
    Block counts: [0]
  _RNvCsaVJ4G3CYWvg_1m4main:
    Hash: 0x01f83c8103c10745
    Counters: 11
    Block counts: [1, 1, 1000, 1, 1, 0, 0, 0, 0, 0, 0]
Instrumentation level: IR  entry_first = 0  instrument_loop_entries = 0
Functions shown: 3
Total functions: 3
Maximum function count: 1000
"""

LD64_MAP = """\
# Path: rel
# Arch: arm64
# Sections:
# Address\tSize    \tSegment\tSection
0x100000858\t0x00032800\t__TEXT\t__text
0x100033058\t0x00000400\t__TEXT\t__const
# Symbols:
# Address\tSize    \tFile  Name
0x100000858\t0x0000000C\t[  1] __RNvCsaVJ4G3CYWvg_1m3hot
0x100000878\t0x00000020\t[  1] __RNvCsaVJ4G3CYWvg_1m4main
0x100000898\t0x00000020\t[  1] __RNvCsaVJ4G3CYWvg_1m4cold
0x100033058\t0x00000010\t[  1] __RNvCsaVJ4G3CYWvg_1m5TABLE
# Dead Stripped Symbols:
#        \tSize    \tFile  Name
<<dead>> \t0x00000008\t[  1] __RNvCsaVJ4G3CYWvg_1m4gone
"""

LLD_MAP = """\
             VMA              LMA     Size Align Out     In      Symbol
          200120           200120       5c     8 .eh_frame
          200120           200120       58     1         t.o:(.eh_frame+0x0)
          2011d0           2011d0       44    16 .text
          2011d0           2011d0        4    16         t.o:(.text.lhot)
          2011d0           2011d0        4     1                 lhot
          2011e0           2011e0        4    16         t.o:(.text.hot)
          2011e0           2011e0        4     1                 hot
          2011f0           2011f0        6    16         t.o:(.text.cold)
          2011f0           2011f0        6     1                 cold
          202218           202218        8     8 .data
          202218           202218        8     8         t.o:(.data.p)
          202218           202218        8     1                 p
"""


class ProfileTests(unittest.TestCase):
    def test_local_function_prefix_is_stripped_and_cold_functions_dropped(self) -> None:
        touched = order_file.parse_profdata_show(PROFDATA_SHOW)
        assert touched == {"_RNvCsaVJ4G3CYWvg_1m3hot": 1000, "_RNvCsaVJ4G3CYWvg_1m4main": 1000}

    def test_older_llvm_colon_separator_is_stripped(self) -> None:
        legacy = "_ZN4main3hot17h0123456789abcdefE"
        assert order_file.profile_symbol(f"src/main.rs:{legacy}") == legacy

    def test_functions_shared_by_more_runs_come_first_then_corpus_order_then_heat(self) -> None:
        per_run = [
            {"start": 5, "pdf": 9, "pdf_warm": 90},
            {"start": 5, "pdf": 1, "office": 3},
            {"start": 1, "office": 1, "image": 7},
        ]
        assert order_file.order_symbols(per_run) == ["start", "pdf", "office", "pdf_warm", "image"]


class LinkMapTests(unittest.TestCase):
    def test_ld64_map_keeps_text_symbols_only(self) -> None:
        symbols, size = order_file.parse_ld64_map(LD64_MAP)
        assert size == 0x32800
        assert symbols == {
            "__RNvCsaVJ4G3CYWvg_1m3hot": 0x100000858,
            "__RNvCsaVJ4G3CYWvg_1m4main": 0x100000878,
            "__RNvCsaVJ4G3CYWvg_1m4cold": 0x100000898,
        }

    def test_lld_map_keeps_text_symbols_only(self) -> None:
        symbols, size = order_file.parse_lld_map(LLD_MAP)
        assert size == 0x44
        assert symbols == {"lhot": 0x2011D0, "hot": 0x2011E0, "cold": 0x2011F0}

    def test_applied_order_has_no_inversions(self) -> None:
        order = ["__RNvCsaVJ4G3CYWvg_1m3hot", "__RNvCsaVJ4G3CYWvg_1m4main", "__RNvCsaVJ4G3CYWvg_1m9inlined"]
        report = order_file.check_map(order, LD64_MAP)
        assert (report.entries, report.applicable, report.inversions) == (3, 2, 0)
        assert (report.workspace_entries, report.workspace_applicable) == (3, 2)
        assert report.ordered_ratio == 1.0

    def test_ignored_order_shows_inversions(self) -> None:
        report = order_file.check_map(["cold", "hot", "lhot"], LLD_MAP)
        assert (report.applicable, report.inversions) == (3, 2)
        assert report.ordered_ratio == 0.0

    def test_sysroot_symbols_do_not_count_toward_the_name_match(self) -> None:
        display = "__RNvXsq_NtCs6eGy9mWPo3p_5alloc6stringNtB5_6StringNtNtCsjSAAZcmtVt9_4core3fmt7Display3fmt"
        assert order_file.symbol_crates(display) == {"alloc", "core"}
        assert not order_file.names_a_workspace_crate(display)
        drop_args = "__RINvNtCsjSAAZcmtVt9_4core3ptr9drop_glueNtNtCsi4LIHK2nfKK_3std3env4ArgsECsaVJ4G3CYWvg_1m"
        assert order_file.names_a_workspace_crate(drop_args)
        assert order_file.symbol_crates("__ZN5xberg7extract17h0123456789abcdefE") == {"xberg"}

    def test_mismatched_names_fail_verification(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            order = Path(scratch) / "order"
            link_map = Path(scratch) / "map"
            order.write_text("__RNvCsOTHERHASH_1m3hot\n__RNvCsOTHERHASH_1m4main\n")
            link_map.write_text(LD64_MAP)
            assert order_file.verify_map(order, link_map, 0.10, 0.99) == 1
            order.write_text("__RNvCsaVJ4G3CYWvg_1m3hot\n__RNvCsaVJ4G3CYWvg_1m4main\n")
            assert order_file.verify_map(order, link_map, 0.10, 0.99) == 0


class CorpusTests(unittest.TestCase):
    def test_corpus_lists_only_files_present_in_the_checkout(self) -> None:
        runs = order_file.read_corpus(order_file.DEFAULT_CORPUS)
        assert runs
        for run in runs:
            assert (order_file.REPO_ROOT / run.path).is_file(), run.path
            assert not run.path.startswith("test_documents/"), "publish legs check out no submodules"
            assert run.content_format in {"plain", "markdown"}


if __name__ == "__main__":
    unittest.main()
