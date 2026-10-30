---
type: Decision
title: Changelog Policy
description: "What belongs in Xberg's user-facing CHANGELOG and what to omit. Use when editing a changelog."
paths:
  - "**/CHANGELOG.md"
  - "docs-site/src/content/docs/changelog.md"
x-ai-rulez:
  kind: rule
  id: changelog-policy
  metadata:
    priority: high
---

Keep `CHANGELOG.md` user-facing. Record Added, Changed, Removed, Fixed, Security, and public API changes that affect
users. Omit CI, tests, generators, dependency maintenance, and internal implementation details unless they change
user-visible behavior, compatibility, packaging, installation, performance, or security.

`docs-site/src/content/docs/changelog.md` must carry every CHANGELOG.md release section byte-for-byte: edit both
(or run `python3 scripts/ci/check-changelog-sync.py --fix`), then `mise run docs:verify:changelog`; CI Lint fails on drift.
