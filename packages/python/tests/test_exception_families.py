"""Check exception-family catches through the built native Python extension."""

import importlib.machinery
import unittest

import xberg
import xberg._xberg as native


class ExceptionFamilyTests(unittest.TestCase):
    """Exercise public family aliases on actual native exceptions."""

    def test_native_io_error_is_catchable_under_both_error_families(self) -> None:
        """Catch one native IoError through both Rust error families."""
        assert any(native.__file__.endswith(suffix) for suffix in importlib.machinery.EXTENSION_SUFFIXES)
        assert xberg.IoError is native.IoError
        assert xberg.LoadError is native.LoadError
        assert xberg.XbergError is native.XbergError
        error = xberg.IoError("shared family contract")
        caught_load = None
        try:
            raise error
        except xberg.LoadError as caught:
            caught_load = caught
        assert caught_load is error
        caught_xberg = None
        try:
            raise error
        except xberg.XbergError as caught:
            caught_xberg = caught
        assert caught_xberg is error
        sibling = xberg.ParseError("load-only sibling")
        assert isinstance(sibling, xberg.LoadError)
        assert not isinstance(sibling, xberg.XbergError)


if __name__ == "__main__":
    unittest.main()
