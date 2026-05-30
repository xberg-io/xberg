# Swift Binding Hand-Edit Audit

Audit of all hand-edits to the Swift binding during the alef-hand-edit cycle (`bd1bef129d..HEAD`).

**Scope**: Commits c8a3dbe70e, 0e57ca4b0e, cbf9e23d2d, 860080e240.

---

## ALEF_GAP: Missing Overload Generation

### Entry 1: JSON-string convenience overloads

**Files & Location**:
- `packages/swift/Sources/Kreuzberg/Kreuzberg.swift`: lines 6390–6450
  - `extractFile(String, String?, String)` positional overload
  - `extractFileSync(String, String?, String)` positional overload
  - `extractBytes(String, String, String)` positional overload (path/UTF-8 fallback)
  - `extractBytesSync(String, String, String)` positional overload (path/UTF-8 fallback)
  - `batchExtractBytesSync(items, config)` with empty JSON default
  - `batchExtractFilesSync(paths, config)` with empty JSON default

**Description**: The alef e2e generator emits extraction calls with positional arguments and JSON-string config: `Kreuzberg.extractFile(path, mimeType, configJson)`. The alef swift templates do not emit these overloads. They are hand-written to bridge the gap between the generator's calling pattern and the generated base functions (which use labeled `config: ExtractionConfig`).

**Label**: `ALEF_GAP`

**Suggested Upstream Fix**: The alef swift templates should emit positional-argument overloads that accept JSON strings for `ExtractionConfig` and its relatives. Pattern:

```swift
// In alef/src/swift/templates/Kreuzberg.swift.jinja2
public func extractFile(_ path: String, _ mimeType: String?, _ configJson: String) throws -> ExtractionResult {
    let config = try extractionConfigFromJson(configJson)
    return try extractFile(path: path, mimeType: mimeType, config: config)
}
```

Also emit similarly for batch functions with default `"{}"` when config is omitted:

```swift
public func batchExtractBytesSync(items: [BatchBytesItem]) throws -> [ExtractionResult] {
    let config = try extractionConfigFromJson("{}")
    return try batchExtractBytesSync(items: items, config: config)
}
```

---

### Entry 2: Path-resolution helper and batch config defaults

**Files & Location**:
- `packages/swift/Sources/Kreuzberg/BridgeRegistrationOverloads.swift`: lines 23–46
- `packages/swift/Sources/Kreuzberg/Kreuzberg.swift`: lines 6417–6418, 6438
  - Calls to `_loadBytesFromPathOrUtf8(content)` in the String-argument `extractBytes` overloads

**Description**: The `_loadBytesFromPathOrUtf8` helper resolves a string as a fixture path (checking CWD, `KREUZBERG_TEST_DOCUMENTS_DIR` env var, and ancestor `test_documents/` or `fixtures/` directories), falling back to raw UTF-8 if no file exists. This matches Python e2e test patterns where fixture paths are embedded in the test calls.

The batch functions with no config argument use `extractionConfigFromJson("{}")` to provide a default empty-object config.

**Label**: `ALEF_GAP`

**Suggested Upstream Fix**: The alef swift templates should emit the `_loadBytesFromPathOrUtf8` helper and wire it into the String-argument overloads:

```swift
// In alef/src/swift/templates/BridgeRegistrationOverloads.swift.jinja2 or as a standalone template
public func _loadBytesFromPathOrUtf8(_ pathOrContent: String) throws -> [UInt8] {
    let fm = FileManager.default
    var roots: [String] = [fm.currentDirectoryPath]
    if let envRoot = ProcessInfo.processInfo.environment["KREUZBERG_TEST_DOCUMENTS_DIR"] {
        roots.append(envRoot)
    }
    var walker = URL(fileURLWithPath: fm.currentDirectoryPath)
    for _ in 0..<16 {
        roots.append(walker.appendingPathComponent("test_documents").path)
        roots.append(walker.appendingPathComponent("fixtures").path)
        let parent = walker.deletingLastPathComponent()
        if parent.path == walker.path { break }
        walker = parent
    }
    let candidates = [pathOrContent] + roots.map { ($0 as NSString).appendingPathComponent(pathOrContent) }
    for path in candidates {
        if fm.fileExists(atPath: path), let data = try? Data(contentsOf: URL(fileURLWithPath: path)) {
            return [UInt8](data)
        }
    }
    return [UInt8](pathOrContent.utf8)
}
```

And update the String-argument overloads:

```swift
public func extractBytes(_ content: String, _ mimeType: String, _ configJson: String) throws -> ExtractionResult {
    let config = try extractionConfigFromJson(configJson)
    let bytes = try _loadBytesFromPathOrUtf8(content)
    return try extractBytes(content: bytes, mimeType: mimeType, config: config)
}
```

---

## ALEF_GAP: Missing Plugin Registration Adapters

### Entry 3: Bridge→Box adapter and register overloads

**Files & Location**:
- `packages/swift/Sources/Kreuzberg/BridgeRegistrationOverloads.swift`: lines 76–98 (register overloads)
- `packages/swift/Sources/Kreuzberg/BridgeRegistrationOverloads.swift`: lines 112–216 (adapter implementations)

**Description**: The alef e2e fixtures call `Kreuzberg.registerOcrBackend(stub)` where `stub` conforms to the lightweight `SwiftOcrBackendBridge` protocol (which only exposes a subset of methods like `supportsLanguage()` and `backendType()`). However, the underlying registration functions expect the full `OcrBackend` protocol (with methods like `processImage`, `initialize`, `shutdown`, etc.).

The hand-written adapters (`_OcrBackendBridgeAdapter`, `_PostProcessorBridgeAdapter`, etc.) wrap the bridge stub and implement the full protocol with sensible defaults: async methods throw with a descriptive error, capability queries return safe defaults (false, empty arrays, no-op initializers).

**Label**: `ALEF_GAP`

**Suggested Upstream Fix**: The alef swift templates should emit register overloads that accept the lightweight Bridge protocols and wrap them in full-protocol adapters. Template pattern:

```swift
// In alef/src/swift/templates/BridgeRegistrationOverloads.swift.jinja2

public func registerOcrBackend(_ bridge: SwiftOcrBackendBridge) throws {
    try registerOcrBackend(SwiftOcrBackendBox(_OcrBackendBridgeAdapter(bridge: bridge)))
}

private final class _OcrBackendBridgeAdapter: OcrBackend {
    private let bridge: any SwiftOcrBackendBridge
    init(bridge: any SwiftOcrBackendBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-ocr-stub" }
    func version() -> String { "0.0.0" }
    func initialize() throws {}
    func shutdown() throws {}
    func processImage(_ image_bytes: [UInt8], config: String) throws -> String {
        throw _BridgeStubError(description: "async bridge processImage cannot be invoked from sync FFI stub")
    }
    // ... other methods with defaults
    func supportsLanguage(_ lang: String) -> Bool { bridge.supportsLanguage(lang: lang) }
    func backendTypeJson() -> String {
        let value = bridge.backendType()
        guard let data = try? JSONEncoder().encode(value),
              let json = String(data: data, encoding: .utf8) else { return "\"Tesseract\"" }
        return json
    }
    // ... other methods
}
```

Emit these overloads for all plugin types: `registerOcrBackend`, `registerPostProcessor`, `registerValidator`, `registerEmbeddingBackend`, `registerDocumentExtractor`, `registerRenderer`.

---

### Entry 4: Unregister name: label overloads

**Files & Location**:
- `packages/swift/Sources/Kreuzberg/BridgeRegistrationOverloads.swift`: lines 50–72

**Description**: The e2e fixtures emit `Kreuzberg.unregisterOcrBackend(name: "...")` calls with a labeled `name:` argument. The generated base functions use positional arguments. Hand-written overloads bridge the gap:

```swift
public func unregisterOcrBackend(name: String) throws {
    try unregisterOcrBackend(name)
}
```

**Label**: `ALEF_GAP`

**Suggested Upstream Fix**: Emit `name:` label overloads for all unregister functions in the alef swift templates:

```swift
public func unregisterOcrBackend(name: String) throws {
    try unregisterOcrBackend(name)
}
```

---

## TEST_FIXTURE: E2E Test Stub Protocol Corrections

### Entry 5: Plugin bridge protocol signature alignment

**Files & Location**:
- `e2e/swift_e2e/Tests/KreuzbergE2ETests/PluginApiTests.swift`: lines 22–83 (across commits c8a3dbe70e and 0e57ca4b0e)

**Description**: The e2e fixtures define stub implementations of the plugin bridge protocols (e.g., `TestStubRegisterDocumentExtractorTraitBridge: SwiftDocumentExtractorBridge`). The alef e2e generator emitted stubs with incorrect signatures:

1. **SwiftDocumentExtractorBridge**: Emitted positional-arg methods returning `InternalDocument` instead of labeled params returning `String`
2. **SwiftEmbeddingBackendBridge**: Emitted `dimensions() -> UInt` instead of `Int`; positional `embed(_:)` instead of labeled
3. **SwiftOcrBackendBridge**: Emitted positional-arg `processImage`, zero-arg `OcrBackendType()` constructors instead of enum cases (`.tesseract`)
4. **SwiftPostProcessorBridge**: Emitted positional-arg `process` and zero-arg `ProcessingStage()` instead of enum cases (`.early`)
5. **SwiftRendererBridge**: Emitted non-throwing `render` instead of `throws`
6. **SwiftValidatorBridge**: Emitted positional-arg `validate` instead of labeled

These were fixed in commit c8a3dbe70e to match the actual generated protocol signatures.

**Label**: `TEST_FIXTURE`

**Suggested Upstream Fix**: The alef e2e swift fixture generator should:

1. Use labeled parameters matching the actual `SwiftXxxBridge` protocol signatures
2. Use correct return types (String not InternalDocument, Int not UInt, Void not return values)
3. Use enum instances (`.tesseract`, `.early`) instead of zero-arg constructors
4. Add `throws` keyword where protocols require it
5. Validate protocol compliance by reading the actual generated bridge files before generating fixture stubs

Example fixture template fix:

```swift
class TestStubRegisterOcrBackendTraitBridge: SwiftOcrBackendBridge {
    var name: String { "register_ocr_backend_trait_bridge" }
    func processImage(image_bytes: Data, config: OcrConfig) async throws -> ExtractionResult {
        try RustBridge.extractionResultFromJson("{}")
    }
    func supportsLanguage(lang: String) -> Bool { false }
    func backendType() -> OcrBackendType { .tesseract }
}
```

---

### Entry 6: Register function call signatures in e2e tests

**Files & Location**:
- `e2e/swift_e2e/Tests/KreuzbergE2ETests/PluginApiTests.swift`: commit 0e57ca4b0e

**Description**: The alef e2e generator initially emitted register calls with labeled arguments (e.g., `registerEmbeddingBackend(backend: ...)`), but the actual generated functions use positional arguments. Commit 0e57ca4b0e corrected these to match:

```swift
// Before (incorrect, alef-generated)
let result = try Kreuzberg.registerEmbeddingBackend(backend: TestStubRegisterEmbeddingBackendTraitBridge())

// After (correct, hand-edited)
let result = try Kreuzberg.registerEmbeddingBackend(TestStubRegisterEmbeddingBackendTraitBridge())
```

**Label**: `TEST_FIXTURE`

**Suggested Upstream Fix**: The alef e2e swift fixture generator should emit register calls with positional arguments, matching the actual function signatures.

---

## TEST_FIXTURE: E2E Test Cleanup & Isolation

### Entry 7: Unregister cleanup after plugin registration tests

**Files & Location**:
- `e2e/swift_e2e/Tests/KreuzbergE2ETests/PluginApiTests.swift`: commit 860080e240
  - Added `try? Kreuzberg.unregisterOcrBackend("swift-bridge-ocr-stub")` and similar after each register test

**Description**: The e2e tests register plugin stubs but did not clean them up. This leaves registered plugins in the registry, affecting subsequent extraction tests (which expect the default tesseract OCR backend to be available via the initialization logic `ensure_ocr_backends_initialized`).

Commit 860080e240 appended unregister calls to each register test using the stub's default names (e.g., `"swift-bridge-ocr-stub"`). This matches the pattern the Python e2e generator already emits.

**Label**: `TEST_FIXTURE`

**Suggested Upstream Fix**: The alef e2e swift fixture generator should append unregister cleanup to each plugin registration test:

```swift
func testRegisterOcrBackendTraitBridge() throws {
    class TestStubRegisterOcrBackendTraitBridge: SwiftOcrBackendBridge {
        var name: String { "register_ocr_backend_trait_bridge" }
        // ...
    }

    let result = try Kreuzberg.registerOcrBackend(TestStubRegisterOcrBackendTraitBridge())
    try? Kreuzberg.unregisterOcrBackend("swift-bridge-ocr-stub")  // <-- Add this
}
```

Document the stub adapter names (e.g., `"swift-bridge-ocr-stub"`) in the BridgeRegistrationOverloads template so the e2e generator can reference them.

---

## ALEF_GAP: Computed-Property Extensions

### Entry 8: Property-accessor aliases for test ergonomics

**Files & Location**:
- `packages/swift/Sources/Kreuzberg/ExtractionResultExtensions.swift` (new file, added commit cbf9e23d2d)

**Description**: The swift-bridge-generated `ExtractionResultRef` type exposes methods like `mimeType()` and `content()`. The alef e2e generator emits test assertions accessing these as properties: `result.mimeType` and `result.content`. Without the computed-property extensions, these fail to compile.

The hand-written extensions add ergonomic aliases:

```swift
extension RustBridge.ExtractionResultRef {
    public var mimeType: String {
        self.mimeType().toString()
    }
    public var content: String {
        self.content().toString()
    }
}
```

**Label**: `ALEF_GAP`

**Suggested Upstream Fix**: The alef swift templates should emit a companion file with computed-property extensions on swift-bridge-generated opaque ref types, allowing property-access syntax in e2e tests. A new template file `ExtractionResultExtensions.swift.jinja2` should emit:

```swift
import RustBridge

// MARK: - Property-access ergonomics for e2e tests
// Provides computed-property aliases for methods on swift-bridge-generated types,
// so callers can write `result.mimeType` rather than `result.mimeType()`.

extension RustBridge.ExtractionResultRef {
    public var mimeType: String {
        self.mimeType().toString()
    }
    public var content: String {
        self.content().toString()
    }
}
```

Make this file hand-editable (not generated) if the set of accessors varies per project; or make it generated if it's stable across all bindings.

---

## ROOT_CAUSE: Bridge Protocol Type Naming Ambiguity

### Entry 9: Qualified type names in test stubs

**Files & Location**:
- `e2e/swift_e2e/Tests/KreuzbergE2ETests/PluginApiTests.swift`: commit 860080e240
  - Line 48: `func backendType() -> Kreuzberg.OcrBackendType { .tesseract }`
  - Line 60: `func processingStage() -> Kreuzberg.ProcessingStage { .early }`

**Description**: The e2e test stubs reference `Kreuzberg.OcrBackendType` and `Kreuzberg.ProcessingStage` with the module prefix to disambiguate from any local declarations. This is necessary because the alef fixture generator may emit type references without qualification, leading to ambiguity.

**Label**: `ROOT_CAUSE`

**Suggested Upstream Fix**: The alef e2e swift fixture generator should always qualify enum types with their module/namespace to avoid ambiguity. Update the fixture template to emit `Kreuzberg.OcrBackendType` instead of bare `OcrBackendType`.

---

## BINDING_BUG: None

No bugs were found in hand-written binding wrapper code. The manual additions (overloads, adapters, extensions) are minimal, well-scoped, and correctly implemented.

---

## Summary

| Category | Count |
| --- | --- |
| ALEF_GAP | 5 |
| TEST_FIXTURE | 3 |
| ROOT_CAUSE | 1 |
| BINDING_BUG | 0 |

---

## Open Questions

1. **ExtractionResultExtensions ownership**: Should this file be generated (and thus regenerated on every `alef` run), or hand-written and committed as part of the binding? If hand-written, it risks diverging from generated types if the swift-bridge codegen changes the method signatures. Consider: (a) include the generator output as a canonical reference in a comment, or (b) generate it and make it stable across alef versions.

2. **Adapter stub names**: The `_OcrBackendBridgeAdapter` class uses hardcoded names like `"swift-bridge-ocr-stub"` returned by `name()`. Should these be configurable, or is the current pattern (fixed names for test cleanup) sufficient? If fixed, document them in alef's swift plugin template.

3. **Bridge protocol vs. full protocol rift**: The lightweight `SwiftOcrBackendBridge` protocol exposes only ~3 methods, while the full `OcrBackend` protocol has ~10. Is this rift intentional (for easier test stubs), or should the bridge and full protocols converge? Current design trades test simplicity for some indirection via adapters.

---

## Suggested Cleanup In-Repo

Before upstreaming fixes to alef templates, consider restructuring locally:

1. **Move `_loadBytesFromPathOrUtf8` to a separate file**: Currently embedded in `BridgeRegistrationOverloads.swift`. Consider a new `TestFixtureHelpers.swift` file to separate test-infrastructure concerns from plugin registration. This makes it clearer which parts are "e2e framework" vs. "production binding".

2. **Document adapter stub names in BridgeRegistrationOverloads**: Add a comment block at the top listing the stub names returned by each adapter (e.g., `"swift-bridge-ocr-stub"`, `"swift-bridge-post-processor-stub"`). This is the contract the e2e generator must know to emit correct cleanup calls.

3. **Consider a PluginBridgeAdapters.swift file**: Move the five adapter implementations (`_OcrBackendBridgeAdapter`, etc.) to a dedicated file for clarity. The current `BridgeRegistrationOverloads.swift` mixes concern: helper functions, label-argument overloads, and adapters. Splitting makes the architectural intent clearer.

4. **Mark computed-property extensions as test-only**: If `ExtractionResultExtensions.swift` is test-specific, add a comment or docstring noting that callers needing property-access syntax should use these extensions. Alternatively, consider whether this is ergonomic enough to expose in production bindings (it likely is).

---

## Conclusion

All hand-edits fall into clear categories: either gaps in alef's swift template generation (missing overloads, adapters, extensions), e2e fixture generation issues (incorrect protocol signatures, missing cleanup), or root-cause type disambiguation. No production bugs detected in the binding logic itself.

**Immediate next step**: File issues or PRs in the alef repo with the suggested template fixes (Entries 1–8), referencing this audit. Once alef templates emit all these patterns, this hand-edit cycle can be eliminated and swift bindings will regenerate cleanly.
