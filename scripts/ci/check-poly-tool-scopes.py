"""Keep clang-format from competing with Poly's Java formatter."""

from __future__ import annotations

import sys
from pathlib import Path

import tomllib

REPO_ROOT = Path(__file__).resolve().parents[2]
EXPECTED_CLANG_FORMAT_FILES = [
    "**/*.c",
    "**/*.cc",
    "**/*.cpp",
    "**/*.cxx",
    "**/*.h",
    "**/*.hh",
    "**/*.hpp",
    "**/*.hxx",
]


def main() -> int:
    config = tomllib.loads((REPO_ROOT / "poly.toml").read_text())
    actual = config["tools"]["clang-format"].get("files")
    if actual != EXPECTED_CLANG_FORMAT_FILES:
        print(
            "clang-format must be restricted to C/C++ source and header globs; "
            f"expected {EXPECTED_CLANG_FORMAT_FILES}, got {actual}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
