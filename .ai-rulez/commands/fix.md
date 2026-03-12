---
priority: high
aliases: [f]
usage: "/fix"
description: "Auto-fix linting, formatting, and common issues"
---
# Fix

Automatically fix as many issues as possible:

1. Run `task format` if available, otherwise run language-specific formatters
2. Run `prek run --all-files` to catch and fix remaining issues
3. Report what was fixed and what still needs manual attention
