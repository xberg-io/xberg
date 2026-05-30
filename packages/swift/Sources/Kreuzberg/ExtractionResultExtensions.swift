import RustBridge

// MARK: - Property-access ergonomics for e2e tests
// Provides computed-property aliases for methods on swift-bridge-generated types,
// so callers can write `result.mimeType` rather than `result.mimeType()`.

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
