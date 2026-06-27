// This file contains adapter bridges that wrap user trait implementations and delegate to the plugin interface.
#nullable enable

using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Xberg;

// MARK: - Internal stub adapters
//
// Each adapter sealed class conforms to the bridge interface `I{TraitName}` by delegating to
// a user-provided implementation instance. The adapter exposes itself to the Rust side via
// P/Invoke as the handle parameter, allowing e2e tests and user code to register implementations.
//
// Adapter stub names (returned by Name property):
// - _OcrBackendBridgeAdapter → "csharp-bridge-ocr_backend-adapter"
// - _PostProcessorBridgeAdapter → "csharp-bridge-post_processor-adapter"
// - _ValidatorBridgeAdapter → "csharp-bridge-validator-adapter"
// - _EmbeddingBackendBridgeAdapter → "csharp-bridge-embedding_backend-adapter"
// - _RendererBridgeAdapter → "csharp-bridge-renderer-adapter"
// - _RerankerBackendBridgeAdapter → "csharp-bridge-reranker_backend-adapter"
//
// These names are used by e2e test cleanup to unregister adapters after each test.

/// <summary>
/// Adapter bridge for OcrBackend trait implementation.
/// Wraps a user-provided IOcrBackend implementation and delegates all method calls.
/// </summary>
public sealed class _OcrBackendBridgeAdapter : IOcrBackend
{
    private readonly IOcrBackend _impl;

    /// <summary>Create an adapter around a user-provided OcrBackend implementation.</summary>
    public _OcrBackendBridgeAdapter(IOcrBackend impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public ExtractionResult ProcessImage(byte[] ImageBytes, OcrConfig Config)
    {
        return _impl.ProcessImage(ImageBytes, Config);
    }

    /// <summary></summary>
    public ExtractionResult ProcessImageFile(string Path, OcrConfig Config)
    {
        return _impl.ProcessImageFile(Path, Config);
    }

    /// <summary></summary>
    public bool SupportsLanguage(string Lang)
    {
        return _impl.SupportsLanguage(Lang);
    }

    /// <summary></summary>
    public OcrBackendType BackendType => _impl.BackendType;

    /// <summary></summary>
    public List<string> SupportedLanguages => _impl.SupportedLanguages;

    /// <summary></summary>
    public bool SupportsTableDetection => _impl.SupportsTableDetection;

    /// <summary></summary>
    public bool SupportsDocumentProcessing => _impl.SupportsDocumentProcessing;

    /// <summary></summary>
    public bool EmitsStructuredMarkdown => _impl.EmitsStructuredMarkdown;

    /// <summary></summary>
    public ExtractionResult ProcessDocument(string Path, OcrConfig Config)
    {
        return _impl.ProcessDocument(Path, Config);
    }

}

/// <summary>
/// Adapter bridge for PostProcessor trait implementation.
/// Wraps a user-provided IPostProcessor implementation and delegates all method calls.
/// </summary>
public sealed class _PostProcessorBridgeAdapter : IPostProcessor
{
    private readonly IPostProcessor _impl;

    /// <summary>Create an adapter around a user-provided PostProcessor implementation.</summary>
    public _PostProcessorBridgeAdapter(IPostProcessor impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public void Process(ExtractionResult Result, ExtractionConfig Config)
    {
        _impl.Process(Result, Config);
    }

    /// <summary></summary>
    public ProcessingStage ProcessingStage => _impl.ProcessingStage;

    /// <summary></summary>
    public bool ShouldProcess(ExtractionResult Result, ExtractionConfig Config)
    {
        return _impl.ShouldProcess(Result, Config);
    }

    /// <summary></summary>
    public ulong EstimatedDurationMs(ExtractionResult Result)
    {
        return _impl.EstimatedDurationMs(Result);
    }

    /// <summary></summary>
    public int Priority => _impl.Priority;

}

/// <summary>
/// Adapter bridge for Validator trait implementation.
/// Wraps a user-provided IValidator implementation and delegates all method calls.
/// </summary>
public sealed class _ValidatorBridgeAdapter : IValidator
{
    private readonly IValidator _impl;

    /// <summary>Create an adapter around a user-provided Validator implementation.</summary>
    public _ValidatorBridgeAdapter(IValidator impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public void Validate(ExtractionResult Result, ExtractionConfig Config)
    {
        _impl.Validate(Result, Config);
    }

    /// <summary></summary>
    public bool ShouldValidate(ExtractionResult Result, ExtractionConfig Config)
    {
        return _impl.ShouldValidate(Result, Config);
    }

    /// <summary></summary>
    public int Priority => _impl.Priority;

}

/// <summary>
/// Adapter bridge for EmbeddingBackend trait implementation.
/// Wraps a user-provided IEmbeddingBackend implementation and delegates all method calls.
/// </summary>
public sealed class _EmbeddingBackendBridgeAdapter : IEmbeddingBackend
{
    private readonly IEmbeddingBackend _impl;

    /// <summary>Create an adapter around a user-provided EmbeddingBackend implementation.</summary>
    public _EmbeddingBackendBridgeAdapter(IEmbeddingBackend impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public ulong Dimensions => _impl.Dimensions;

    /// <summary></summary>
    public List<List<float>> Embed(List<string> Texts)
    {
        return _impl.Embed(Texts);
    }

}

/// <summary>
/// Adapter bridge for Renderer trait implementation.
/// Wraps a user-provided IRenderer implementation and delegates all method calls.
/// </summary>
public sealed class _RendererBridgeAdapter : IRenderer
{
    private readonly IRenderer _impl;

    /// <summary>Create an adapter around a user-provided Renderer implementation.</summary>
    public _RendererBridgeAdapter(IRenderer impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public string Render(string Doc)
    {
        return _impl.Render(Doc);
    }

}

/// <summary>
/// Adapter bridge for RerankerBackend trait implementation.
/// Wraps a user-provided IRerankerBackend implementation and delegates all method calls.
/// </summary>
public sealed class _RerankerBackendBridgeAdapter : IRerankerBackend
{
    private readonly IRerankerBackend _impl;

    /// <summary>Create an adapter around a user-provided RerankerBackend implementation.</summary>
    public _RerankerBackendBridgeAdapter(IRerankerBackend impl)
    {
        _impl = impl ?? throw new ArgumentNullException(nameof(impl));
    }

    // MARK: - Plugin lifecycle (if present)

    /// <summary>Get the plugin name.</summary>
    public string Name => _impl.Name;

    /// <summary>Get the plugin version.</summary>
    public string Version => _impl.Version;

    /// <summary>Initialize the plugin.</summary>
    public void Initialize() => _impl.Initialize();

    /// <summary>Shut down the plugin.</summary>
    public void Shutdown() => _impl.Shutdown();

    // MARK: - Trait methods

    /// <summary></summary>
    public List<float> Rerank(string Query, List<string> Documents)
    {
        return _impl.Rerank(Query, Documents);
    }

}
