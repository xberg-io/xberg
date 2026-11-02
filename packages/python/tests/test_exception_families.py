"""Check exception-family catches through the built native Python extension."""

import importlib.machinery
import unittest

import xberg
import xberg._xberg as native


class ExceptionFamilyTests(unittest.TestCase):
    def test_native_io_error_is_catchable_under_both_error_families(self) -> None:
        assert any(native.__file__.endswith(suffix) for suffix in importlib.machinery.EXTENSION_SUFFIXES)
        assert xberg.IoError is native.IoError
        assert xberg.LoadError is native.LoadError
        assert xberg.XbergError is native.XbergError
        error = xberg.IoError("shared family contract")
        caught_families = []
        for family in (xberg.LoadError, xberg.XbergError):
            try:
                raise error
            except family as caught:
                assert caught is error
                caught_families.append(family)
        assert caught_families == [xberg.LoadError, xberg.XbergError]
        sibling = xberg.ParseError("load-only sibling")
        assert isinstance(sibling, xberg.LoadError)
        assert not isinstance(sibling, xberg.XbergError)


if __name__ == "__main__":
    unittest.main()
