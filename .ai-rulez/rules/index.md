# Concepts

- [Api Compatibility](api-compatibility.md) - Xberg public API compatibility: semver, deprecation window, CHANGELOG entries, FFI-friendly public types. Use when changing a public API.
- [Async And Concurrency](async-and-concurrency.md) - Xberg async rules: tokio-only extraction paths, spawn_blocking for CPU work, Send + Sync types, timeouts. Use when editing core async code.
- [Changelog Policy](changelog-policy.md) - What belongs in Xberg's user-facing CHANGELOG and what to omit. Use when editing a changelog.
- [Extraction Measurement Hygiene](extraction-measurement-hygiene.md) - Xberg extraction A/B and sweep hygiene: per-run cache isolation, OCR no-cache, CLI JSON nesting, timings. Use when measuring extraction output.
- [Extraction Safety](extraction-safety.md) - Xberg extraction safety: SecurityLimits, MIME validation, fallback eligibility, partial results, error text. Use when editing extractors or core.
- [Measurement Discipline](measurement-discipline.md) - Judging Xberg quality changes: absolute counts over ratios, tail vs population, bracketing preserved binaries. Use when comparing extraction quality.
- [Plugin Bundles](plugin-bundles.md) - The agent plugin and root skills/ are generated from plugin/.ai-rulez: what to edit, how to regenerate and verify. Use when editing plugin/ or skills/.
