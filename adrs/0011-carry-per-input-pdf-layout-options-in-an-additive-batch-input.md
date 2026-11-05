---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Carry per-input PDF layout options in an additive batch input

## Context and Problem Statement

A batch can request native PDF layout for every input or for none, but cannot mix the two. Adding a field to the public Rust `FileExtractionConfig` would break callers that construct it with exhaustive literals.

## Considered Options

* Add an optional field to `FileExtractionConfig` and require existing callers to change.
* Add an options wrapper and a separate batch entry point while retaining the existing input and config types.

## Decision Outcome

Chosen option: "Add an options wrapper and a separate batch entry point", because the existing Rust literals, input format, and global default remain compatible. `ExtractInputWithOptions` wraps the existing input and adds an optional layout override. Omission inherits the batch setting; explicit true or false wins for that input. Resolve the setting before processing or computing cache dependencies.

The Swift binding uses the JSON batch entry point. Its generated vector bridge consumes opaque wrappers, which makes passing the same wrapper twice unsafe. The JSON entry point validates and parses the same input options into independent Rust values.

### Consequences

* Good, because an existing caller needs no changes and each PDF can select layout independently.
* Good, because the options wrapper can grow without changing exhaustive literals for the legacy type.
* Bad, because callers opt in through a separate entry point and Swift serializes its inputs as JSON.

### Confirmation

Compile an exhaustive legacy config literal. Exercise true, false, and omitted options under both batch defaults, including cache lookups. Check the generated Swift bridge and pass the same input twice through the JSON entry point.

## More Information

* [Issue #2367](https://github.com/xberg-io/xberg/issues/2367)
