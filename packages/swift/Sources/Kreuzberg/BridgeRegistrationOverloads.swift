// Convenience overloads matching the alef e2e generator's call shapes.
//
// The e2e fixtures call `Kreuzberg.register<plugin>(bridgeStub)` against the
// slim `Swift<Plugin>Bridge` protocols, but the underlying registration
// functions take the full `Swift<Plugin>Box` (which wraps the typed
// `<Plugin>` protocol). These overloads wrap the bridge in a minimal
// stub adapter so registration succeeds; the stub adapters use sensible
// defaults for the methods the bridge does not expose.
//
// Unregister overloads accept the `name:` argument label that the e2e
// fixtures use; they forward to the positional-arg base function.

import Foundation
import RustBridge

// MARK: - Path/UTF-8 helper used by the String overloads of extractBytes(Sync).

/// Treat the input as a filesystem path first (resolved against the test
/// fixtures directory if relative); fall back to the raw UTF-8 bytes if no
/// such file exists. The alef e2e generator emits fixture paths into
/// `extract_bytes` calls, but third-party callers may still want to pass
/// inline string content.
public func _loadBytesFromPathOrUtf8(_ pathOrContent: String) throws -> [UInt8] {
    let fm = FileManager.default
    var roots: [String] = [
        fm.currentDirectoryPath,
    ]
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

// MARK: - Unregister name: label overloads

public func unregisterOcrBackend(name: String) throws {
    try unregisterOcrBackend(name)
}

public func unregisterPostProcessor(name: String) throws {
    try unregisterPostProcessor(name)
}

public func unregisterValidator(name: String) throws {
    try unregisterValidator(name)
}

public func unregisterEmbeddingBackend(name: String) throws {
    try unregisterEmbeddingBackend(name)
}

public func unregisterDocumentExtractor(name: String) throws {
    try unregisterDocumentExtractor(name)
}

public func unregisterRenderer(name: String) throws {
    try unregisterRenderer(name)
}

// MARK: - Bridge → Box register overloads

public func registerOcrBackend(_ bridge: SwiftOcrBackendBridge) throws {
    try registerOcrBackend(SwiftOcrBackendBox(_OcrBackendBridgeAdapter(bridge: bridge)))
}

public func registerPostProcessor(_ bridge: SwiftPostProcessorBridge) throws {
    try registerPostProcessor(SwiftPostProcessorBox(_PostProcessorBridgeAdapter(bridge: bridge)))
}

public func registerValidator(_ bridge: SwiftValidatorBridge) throws {
    try registerValidator(SwiftValidatorBox(_ValidatorBridgeAdapter(bridge: bridge)))
}

public func registerEmbeddingBackend(_ bridge: SwiftEmbeddingBackendBridge) throws {
    try registerEmbeddingBackend(SwiftEmbeddingBackendBox(_EmbeddingBackendBridgeAdapter(bridge: bridge)))
}

public func registerDocumentExtractor(_ bridge: SwiftDocumentExtractorBridge) throws {
    try registerDocumentExtractor(SwiftDocumentExtractorBox(_DocumentExtractorBridgeAdapter(bridge: bridge)))
}

public func registerRenderer(_ bridge: SwiftRendererBridge) throws {
    try registerRenderer(SwiftRendererBox(_RendererBridgeAdapter(bridge: bridge)))
}

// MARK: - Internal stub adapters
//
// Each adapter conforms to the full plugin protocol expected by the
// `Swift<Plugin>Box` wrapper and delegates only the methods the bridge
// exposes. The remaining methods use safe defaults: register/initialize/
// shutdown are no-ops, processing entrypoints throw, capability queries
// report false/empty.
//
// Adapter stub names (returned by name() method):
// - _OcrBackendBridgeAdapter → "swift-bridge-ocr-stub"
// - _PostProcessorBridgeAdapter → "swift-bridge-post-processor-stub"
// - _ValidatorBridgeAdapter → "swift-bridge-validator-stub"
// - _EmbeddingBackendBridgeAdapter → "swift-bridge-embedding-stub"
// - _DocumentExtractorBridgeAdapter → "swift-bridge-document-extractor-stub"
// - _RendererBridgeAdapter → "swift-bridge-renderer-stub"
//
// These names are used by e2e test cleanup to unregister stubs after each test.

private struct _BridgeStubError: Error, CustomStringConvertible {
    let description: String
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
    func processImageFile(path: String, config: String) throws -> String {
        throw _BridgeStubError(description: "async bridge processImageFile cannot be invoked from sync FFI stub")
    }
    func supportsLanguage(_ lang: String) -> Bool { bridge.supportsLanguage(lang: lang) }
    func backendTypeJson() -> String {
        let value = bridge.backendType()
        guard let data = try? JSONEncoder().encode(value),
              let json = String(data: data, encoding: .utf8) else {
            return "\"Tesseract\""
        }
        return json
    }
    func supportedLanguages() -> [String] { [] }
    func supportsTableDetection() -> Bool { false }
    func supportsDocumentProcessing() -> Bool { false }
    func processDocument(path: String, config: String) throws -> String {
        throw _BridgeStubError(description: "async bridge processDocument cannot be invoked from sync FFI stub")
    }
}

private final class _PostProcessorBridgeAdapter: PostProcessor {
    private let bridge: any SwiftPostProcessorBridge
    init(bridge: any SwiftPostProcessorBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-post-processor-stub" }
    func version() -> String { "0.0.0" }
    func initialize() throws {}
    func shutdown() throws {}
    func processJson(result: String, config: String) throws -> String { result }
    func processingStageJson() -> String {
        let value = bridge.processingStage()
        guard let data = try? JSONEncoder().encode(value),
              let json = String(data: data, encoding: .utf8) else {
            return "\"PostExtraction\""
        }
        return json
    }
    func shouldProcess(result: String, config: String) -> Bool { false }
    func estimatedDurationMs(result: String) -> UInt64 { 0 }
    func priority() -> Int32 { 50 }
}

private final class _ValidatorBridgeAdapter: Validator {
    private let bridge: any SwiftValidatorBridge
    init(bridge: any SwiftValidatorBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-validator-stub" }
    func version() -> String { "0.0.0" }
    func initialize() throws {}
    func shutdown() throws {}
    func validate(result: String, config: String) throws {}
    func shouldValidate(result: String, config: String) -> Bool { false }
    func priority() -> Int32 { 50 }
}

private final class _EmbeddingBackendBridgeAdapter: EmbeddingBackend {
    private let bridge: any SwiftEmbeddingBackendBridge
    init(bridge: any SwiftEmbeddingBackendBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-embedding-stub" }
    func version() -> String { "0.0.0" }
    func initialize() throws {}
    func shutdown() throws {}
    func dimensions() -> UInt {
        let value = bridge.dimensions()
        return value > 0 ? UInt(value) : 1
    }
    func embed(_ texts: [String]) throws -> String {
        throw _BridgeStubError(description: "async bridge embed cannot be invoked from sync FFI stub")
    }
}

private final class _DocumentExtractorBridgeAdapter: DocumentExtractor {
    private let bridge: any SwiftDocumentExtractorBridge
    init(bridge: any SwiftDocumentExtractorBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-document-extractor-stub" }
    func version() -> String { "0.0.0" }
    func extractBytes(content: [UInt8], mimeType: String, config: String) throws -> String {
        throw _BridgeStubError(description: "async bridge extractBytes cannot be invoked from sync FFI stub")
    }
    func supportedMimeTypes() -> [String] { bridge.supportedMimeTypes() }
}

private final class _RendererBridgeAdapter: Renderer {
    private let bridge: any SwiftRendererBridge
    init(bridge: any SwiftRendererBridge) { self.bridge = bridge }

    func name() -> String { "swift-bridge-renderer-stub" }
    func version() -> String { "0.0.0" }
    func render(doc: String) throws -> String {
        try bridge.render(doc: doc)
    }
}
