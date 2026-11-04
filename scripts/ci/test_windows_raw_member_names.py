"""Reject Windows path aliases before interpreting raw artifact membership."""

import unittest

import test_windows_raw_audit_boundaries as fixtures
from test_windows_wheel_closure import expected_error


class WindowsMemberNames(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = fixtures.RawMemberBoundaries("runTest")
        self.fx.setUp()
        self.addCleanup(self.fx.doCleanups)

    def reject(self, member: str) -> None:
        artifact = self.fx.artifact(".zip", member)
        with expected_error(ValueError, "unsafe artifact member"):
            self.fx.fx.audit(artifact)

    def test_trailing_dot_cannot_alias_a_native_file(self) -> None:
        self.reject("xberg.dll.")

    def test_trailing_space_cannot_alias_a_native_file(self) -> None:
        self.reject("xberg.dll ")

    def test_reserved_device_filename_is_not_an_archive_payload(self) -> None:
        self.reject("NUL.txt")

    def test_reserved_device_directory_is_not_an_archive_payload(self) -> None:
        self.reject("con/payload.txt")

    def test_superscript_com_and_lpt_device_names_are_reserved(self) -> None:
        for device in ("com", "LPT"):
            for digit in "¹²³":
                with self.subTest(device=device, digit=digit):
                    self.reject(f"{device}{digit}/payload.txt")

    def test_control_characters_are_not_windows_member_names(self) -> None:
        self.reject("notice\n.txt")

    def test_windows_forbidden_filename_characters_are_not_members(self) -> None:
        self.reject("a|b.txt")


if __name__ == "__main__":
    unittest.main()
