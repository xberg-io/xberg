"""A simple greeting module."""

import os
from pathlib import Path


def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"


class Greeter:
    """A greeter class."""

    def __init__(self, prefix: str = "Hello") -> None:
        self.prefix = prefix

    def greet(self, name: str) -> str:
        return f"{self.prefix}, {name}!"
