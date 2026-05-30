import RustBridge

// MARK: - Property-access ergonomics for e2e tests
//
// This file provides computed-property aliases for methods on swift-bridge-generated types,
// allowing callers to write `result.mimeType` rather than `result.mimeType()`.
// These extensions are especially useful in e2e test assertions where the alef
// fixture generator emits property-access syntax.
//
// Although these are primarily for test convenience, they are part of the public API
// and can be used in production code for more ergonomic access to extraction results.

extension RustBridge.ExtractionResultRef {
    /// Computed-property alias for `mimeType()` method.
    public var mimeType: String {
        self.mimeType().toString()
    }

    /// Computed-property alias for `content()` method.
    public var content: String {
        self.content().toString()
    }

}

// ExtractionResultRefMut and ExtractionResult inherit the extensions automatically
