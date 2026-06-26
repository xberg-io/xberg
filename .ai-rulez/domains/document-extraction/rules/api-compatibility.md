---
priority: high
---

- Follow semantic versioning — breaking changes require major version bump
- Document all notable changes in CHANGELOG.md under the `## [Unreleased]` heading, in the matching Keep a Changelog subsection (`### Added` / `### Changed` / `### Fixed` / `### Removed` / `### Security`). Create the subsection if it is missing. Never add new entries under an already-released `## [x.y.z]` section — those are frozen history.
- Maintain backward compatibility for at least one minor version before removing deprecated APIs
- All public types must be FFI-friendly or have FFI-compatible equivalents
- Version in Cargo.toml is the single source of truth for all binding packages
