// Hand-authored Swift-side adapter classes for the inbound plugin trait bridge.
//
// The Rust crate `kreuzberg-swift` declares `extern "Swift" type Swift{Trait}Box` for each
// kreuzberg plugin trait — Rust calls into Swift via these handles whenever the host needs
// to drive a registered Swift plugin. swift-bridge looks up the Swift classes by name and
// uses `Unmanaged<T>.passRetained` to bridge ARC across the FFI boundary.
//
// This file is *not* alef-generated: alef emits the Rust side of the bridge plus the FFI
// shim signatures, but the user-facing Swift API (the protocols you adopt, plus the box
// classes that adapt those protocols to the FFI) lives here so users can iterate without
// needing to regenerate the bindings.
//
// Marshalling strategy mirrors the Rust side:
//
//   - Primitives, `String`, `[UInt8]`, and `[String]` pass through directly.
//   - Complex types (`OcrConfig`, `ExtractionConfig`, `ExtractionResult`, …) are exchanged
//     as JSON-encoded strings and decoded via `Codable`.
//   - Fallible methods return a JSON envelope (`{"ok": <value>}` / `{"err": "<message>"}`)
//     because swift-bridge 0.1.59 cannot bridge `Result<RustString, RustString>` correctly.
//
// To register a Swift plugin:
//
//   ```swift
//   final class MyOcrBackend: OcrBackend { /* … */ }
//   try Kreuzberg.registerOcrBackend(MyOcrBackend())
//   ```

import Foundation
import RustBridge

// MARK: - JSON envelope helpers

/// JSON envelope used by every fallible Swift trait method. Carries `Ok(T)` as
/// `{"ok": <serialised T>}` and `Err(String)` as `{"err": "<message>"}`. Mirrors the Rust
/// `InboundEnvelope<T>` enum in the alef-generated bridge.
private enum InboundEnvelope<T: Encodable>: Encodable {
    case ok(T)
    case err(String)

    enum CodingKeys: String, CodingKey { case ok, err }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ok(let value): try container.encode(value, forKey: .ok)
        case .err(let message): try container.encode(message, forKey: .err)
        }
    }
}

/// Encode a successful `()` result as `{"ok":null}`.
private func encodeOkVoidEnvelope() -> RustString {
    return RustString("{\"ok\":null}")
}

/// Encode a successful `T: Encodable` result as `{"ok": <T>}`. Failures during encoding
/// are caught and converted into an error envelope so the Rust side never sees a panic.
private func encodeOkEnvelope<T: Encodable>(_ value: T) -> RustString {
    do {
        let payload = InboundEnvelope.ok(value)
        let data = try JSONEncoder().encode(payload)
        return RustString(String(data: data, encoding: .utf8) ?? "{\"err\":\"swift: invalid utf8 in envelope\"}")
    } catch {
        return encodeErrEnvelope("swift: failed to encode ok envelope: \(error)")
    }
}

/// Encode a failure as `{"err": "<message>"}`.
private func encodeErrEnvelope(_ message: String) -> RustString {
    let escaped = message.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"")
    return RustString("{\"err\":\"\(escaped)\"}")
}

/// Decode a JSON-encoded payload into a `Decodable` type. Throws on failure.
private func decodeJson<T: Decodable>(_ json: String, as type: T.Type) throws -> T {
    let data = json.data(using: .utf8) ?? Data()
    return try JSONDecoder().decode(type, from: data)
}

// MARK: - OcrBackend

/// Swift-native protocol mirroring the Rust `OcrBackend` plugin trait.
///
/// Conforming classes must be reference types (`AnyObject`) so the Rust side can hold a
/// stable retained reference. Complex parameter and return values are exchanged as JSON
/// strings via `Codable`-compatible types.
public protocol OcrBackend: AnyObject {
    /// Stable plugin name used as the registry key.
    func name() -> String
    /// Plugin version (semver-style string).
    func version() -> String
    /// Initialise the plugin. Throw to abort registration.
    func initialize() throws
    /// Shutdown hook. Throw to log a non-fatal cleanup error.
    func shutdown() throws
    /// Process a raw image buffer. `config` is a serialised `kreuzberg::OcrConfig`;
    /// the return value must be a serialised `kreuzberg::ExtractionResult`.
    func processImage(_ image_bytes: [UInt8], config: String) throws -> String
    /// Process an image file at the given path. `config` is a serialised `OcrConfig`.
    func processImageFile(path: String, config: String) throws -> String
    /// Whether the plugin supports the given language.
    func supportsLanguage(_ lang: String) -> Bool
    /// JSON-encoded `OcrBackendType`.
    func backendTypeJson() -> String
    /// Languages supported by this backend.
    func supportedLanguages() -> [String]
    /// Whether the plugin can detect tables.
    func supportsTableDetection() -> Bool
    /// Whether the plugin can process whole documents (vs single images).
    func supportsDocumentProcessing() -> Bool
    /// Process an entire document. `config` is a serialised `OcrConfig`; the return
    /// value must be a serialised `ExtractionResult`.
    func processDocument(path: String, config: String) throws -> String
}

/// FFI adapter class for `OcrBackend`. Rust looks up `SwiftOcrBackendBox` by name from
/// the `extern "Swift"` block and dispatches calls through `Unmanaged<T>.fromOpaque(...)`.
public final class SwiftOcrBackendBox {
    private let inner: OcrBackend

    public init(_ inner: OcrBackend) {
        self.inner = inner
    }

    public func alef_name() -> RustString { RustString(inner.name()) }
    public func alef_version() -> RustString { RustString(inner.version()) }

    public func alef_initialize() -> RustString {
        do { try inner.initialize(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }
    public func alef_shutdown() -> RustString {
        do { try inner.shutdown(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_process_image(image_bytes: RustVec<UInt8>, config: RustString) -> RustString {
        do {
            let bytes = Array(image_bytes)
            let result = try inner.processImage(bytes, config: config.toString())
            return RustString("{\"ok\":\(result)}")
        } catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_process_image_file(path: RustString, config: RustString) -> RustString {
        do {
            let result = try inner.processImageFile(path: path.toString(), config: config.toString())
            return RustString("{\"ok\":\(result)}")
        } catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_supports_language(lang: RustString) -> Bool {
        return inner.supportsLanguage(lang.toString())
    }

    public func alef_backend_type() -> RustString {
        return RustString(inner.backendTypeJson())
    }

    public func alef_supported_languages() -> RustVec<RustString> {
        let languages = inner.supportedLanguages()
        let vec = RustVec<RustString>()
        for lang in languages { vec.push(value: RustString(lang)) }
        return vec
    }

    public func alef_supports_table_detection() -> Bool { inner.supportsTableDetection() }
    public func alef_supports_document_processing() -> Bool { inner.supportsDocumentProcessing() }

    public func alef_process_document(path: RustString, config: RustString) -> RustString {
        do {
            let result = try inner.processDocument(path: path.toString(), config: config.toString())
            return RustString("{\"ok\":\(result)}")
        } catch { return encodeErrEnvelope("\(error)") }
    }
}

// MARK: - PostProcessor

/// Swift-native protocol mirroring the Rust `PostProcessor` plugin trait.
public protocol PostProcessor: AnyObject {
    func name() -> String
    func version() -> String
    func initialize() throws
    func shutdown() throws
    /// Process a serialised `ExtractionResult` (mutable on the Rust side, but we ferry
    /// the result as JSON in/out to avoid round-tripping references through the FFI).
    /// The return value is the post-processed `ExtractionResult` JSON.
    func processJson(result: String, config: String) throws -> String
    /// JSON-encoded `ProcessingStage`.
    func processingStageJson() -> String
    func shouldProcess(result: String, config: String) -> Bool
    func estimatedDurationMs(result: String) -> UInt64
    func priority() -> Int32
}

public final class SwiftPostProcessorBox {
    private let inner: PostProcessor
    public init(_ inner: PostProcessor) { self.inner = inner }

    public func alef_name() -> RustString { RustString(inner.name()) }
    public func alef_version() -> RustString { RustString(inner.version()) }
    public func alef_initialize() -> RustString {
        do { try inner.initialize(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }
    public func alef_shutdown() -> RustString {
        do { try inner.shutdown(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_process(result: RustString, config: RustString) -> RustString {
        do {
            let result = try inner.processJson(result: result.toString(), config: config.toString())
            return RustString("{\"ok\":\(result)}")
        } catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_processing_stage() -> RustString { RustString(inner.processingStageJson()) }
    public func alef_should_process(result: RustString, config: RustString) -> Bool {
        inner.shouldProcess(result: result.toString(), config: config.toString())
    }
    public func alef_estimated_duration_ms(result: RustString) -> UInt64 {
        inner.estimatedDurationMs(result: result.toString())
    }
    public func alef_priority() -> Int32 { inner.priority() }
}

// MARK: - Validator

/// Swift-native protocol mirroring the Rust `Validator` plugin trait.
public protocol Validator: AnyObject {
    func name() -> String
    func version() -> String
    func initialize() throws
    func shutdown() throws
    /// Validate an `ExtractionResult` (passed as JSON). Throw to surface a validation error.
    func validate(result: String, config: String) throws
    func shouldValidate(result: String, config: String) -> Bool
    func priority() -> Int32
}

public final class SwiftValidatorBox {
    private let inner: Validator
    public init(_ inner: Validator) { self.inner = inner }

    public func alef_name() -> RustString { RustString(inner.name()) }
    public func alef_version() -> RustString { RustString(inner.version()) }
    public func alef_initialize() -> RustString {
        do { try inner.initialize(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }
    public func alef_shutdown() -> RustString {
        do { try inner.shutdown(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_validate(result: RustString, config: RustString) -> RustString {
        do {
            try inner.validate(result: result.toString(), config: config.toString())
            return encodeOkVoidEnvelope()
        } catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_should_validate(result: RustString, config: RustString) -> Bool {
        inner.shouldValidate(result: result.toString(), config: config.toString())
    }
    public func alef_priority() -> Int32 { inner.priority() }
}

// MARK: - EmbeddingBackend

/// Swift-native protocol mirroring the Rust `EmbeddingBackend` plugin trait.
public protocol EmbeddingBackend: AnyObject {
    func name() -> String
    func version() -> String
    func initialize() throws
    func shutdown() throws
    /// Embedding dimensions reported by the backend.
    func dimensions() -> UInt
    /// Embed a batch of texts. Returns a JSON-encoded `Vec<Vec<f32>>` (outer Vec = batch
    /// items, inner Vec = embedding components).
    func embed(_ texts: [String]) throws -> String
}

public final class SwiftEmbeddingBackendBox {
    private let inner: EmbeddingBackend
    public init(_ inner: EmbeddingBackend) { self.inner = inner }

    public func alef_name() -> RustString { RustString(inner.name()) }
    public func alef_version() -> RustString { RustString(inner.version()) }
    public func alef_initialize() -> RustString {
        do { try inner.initialize(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }
    public func alef_shutdown() -> RustString {
        do { try inner.shutdown(); return encodeOkVoidEnvelope() }
        catch { return encodeErrEnvelope("\(error)") }
    }

    public func alef_dimensions() -> UInt { inner.dimensions() }
    public func alef_embed(texts: RustVec<RustString>) -> RustString {
        do {
            // RustVec<RustString> iteration yields RustStringRef (borrowed). Use the
            // String(...) initializer that swift-bridge provides on RustStringRef to
            // copy the text out into an owned Swift String.
            var strings: [String] = []
            let count = texts.len()
            var idx: UInt = 0
            while idx < count {
                strings.append(texts.get(index: idx)!.as_str().toString())
                idx += 1
            }
            let result = try inner.embed(strings)
            return RustString("{\"ok\":\(result)}")
        } catch { return encodeErrEnvelope("\(error)") }
    }
}
