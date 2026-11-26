"""The tessdata_best manifest the best images verify against must follow pins.rs."""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parents[2]
sys.path.insert(0, str(SCRIPT_DIR))

import generate_best_manifest as manifest  # noqa: E402

DOCKERFILES = (ROOT / "docker/Dockerfile.core", ROOT / "docker/Dockerfile.full")
DEBIAN_LANGUAGE_PACKAGE = re.compile(r"^\s+tesseract-ocr-([a-z-]+) \\$", re.MULTILINE)


def value_error_message(function: Callable[[], object]) -> str:
    """Return the message of the ValueError ``function`` raises; fail when it raises none."""
    try:
        function()
    except ValueError as error:
        return str(error)
    raise AssertionError("expected a ValueError")


def checked_in_stale_files(pins_text: str) -> list[str]:
    return manifest.stale_files(
        pins_text,
        manifest.REVISION_PATH.read_text(encoding="utf-8"),
        manifest.MANIFEST_PATH.read_text(encoding="utf-8"),
    )


class ManifestTest(unittest.TestCase):
    def setUp(self) -> None:
        self.pins = manifest.PINS_PATH.read_text(encoding="utf-8")

    def test_checked_in_files_match_pins(self) -> None:
        assert checked_in_stale_files(self.pins) == []

    def test_manifest_lists_every_image_language_and_the_license(self) -> None:
        names = [line.split("  ", 1)[1] for line in manifest.MANIFEST_PATH.read_text().splitlines()]
        expected = [f"{language}.traineddata" for language in manifest.IMAGE_LANGUAGES] + ["LICENSE"]
        assert names == expected
        assert len(names) == 17
        assert {"osd.traineddata", "equ.traineddata", "chi_sim.traineddata", "jpn_vert.traineddata"} <= set(names)

    def test_revision_is_the_pinned_best_revision(self) -> None:
        revision, _ = manifest.parse_pins(self.pins)
        assert re.fullmatch(r"[0-9a-f]{40}", revision)
        assert manifest.REVISION_PATH.read_text() == f"{revision}\n"

    def test_a_changed_pin_makes_the_manifest_stale(self) -> None:
        _, digests = manifest.parse_pins(self.pins)
        changed = self.pins.replace(digests["eng"], "0" * 64, 1)
        assert changed != self.pins
        assert checked_in_stale_files(changed) == [manifest.MANIFEST_PATH.name]

    def test_a_new_revision_needs_a_pinned_license_digest(self) -> None:
        revision, _ = manifest.parse_pins(self.pins)
        moved = self.pins.replace(revision, "f" * 40)
        assert "no LICENSE digest pinned" in value_error_message(lambda: manifest.render(moved))

    def test_a_dropped_language_pin_is_rejected(self) -> None:
        without_kor = re.sub(r'(?m)^\s+\("kor", .*\n', "", self.pins)
        assert without_kor != self.pins
        assert value_error_message(lambda: manifest.render(without_kor)).endswith("does not pin kor")

    def test_best_set_covers_every_debian_language_the_images_install(self) -> None:
        for dockerfile in DOCKERFILES:
            packages = DEBIAN_LANGUAGE_PACKAGE.findall(dockerfile.read_text())
            languages = {package.replace("-", "_") for package in packages}
            assert len(languages) == 13, (dockerfile.name, sorted(languages))
            assert languages <= set(manifest.IMAGE_LANGUAGES), (dockerfile.name, sorted(languages))


if __name__ == "__main__":
    unittest.main()
