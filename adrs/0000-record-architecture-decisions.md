---
status: accepted
date: 2026-10-30
decision-makers: ["Na'aman Hirschfeld"]
---

# Record architecture decisions

## Context and Problem Statement

xberg's lasting choices (how each language reaches the Rust core, which PDF engine is the default, what stays off the
binding surface) are recorded only in commit messages, the CHANGELOG, `.ai-rulez/` rules and skills, and comments in
`Cargo.toml` and `alef.toml`. None of these says what was rejected, and nothing marks when a decision changes. The
organization adopted a MADR log checked by adrlint in
[org ADR-0000 (Record architecture decisions)](https://github.com/xberg-io/agent-conventions/blob/main/adrs/0000-record-architecture-decisions.md),
which expects each repository to keep its own decisions in its own log.

## Considered Options

* A repository log of MADR records in `adrs/`, checked by adrlint, as org ADR-0000 sets out

No alternatives were recorded.

## Decision Outcome

Chosen option: "A repository log of MADR records in `adrs/`, checked by adrlint, as org ADR-0000 sets out", because
it gives each xberg decision one immutable record that a later change supersedes or amends instead of overwriting.

Records follow MADR 4 as `adrlint new` generates it, numbered from 0000. Records 0001 onward are seeds: decisions
already in force when the log started, dated to the commit that made each one. Org-wide decisions (the polyrepo, alef
bindings, licensing, the mise baseline and tasks, poly, ai-rulez, action pins, runner labels, the docs theme,
feature-flag names, language tooling, Renovate, shared assets) are cited as `org ADR-NNNN`, not restated. The index is
`docs-site/src/content/docs/reference/architecture-decisions.md`, outside the log.

### Consequences

* Good, because a reviewer can find why a binding, engine or boundary is the way it is, and what it replaced.
* Good, because adrlint checks the shape and numbering of every record, and, once the log is locked, its
  immutability.
* Bad, because the seed records are written after the fact, so their context is limited to what the history shows.

### Confirmation

`.adrlint.toml` sets `dir = "adrs"` and `start = 0`. `adrlint lint` and `adrlint format --check` exit 0 on `adrs/`,
and the poly hooks `adrlint-lint`, `adrlint-format-check` and `adrlint-lock-check` run them on commit.

## More Information

* [MADR](https://adr.github.io/madr/)
* [adrlint](https://github.com/Goldziher/adrlint)
* [Org decision log](https://github.com/xberg-io/agent-conventions/blob/main/docs/adr-index.md)
