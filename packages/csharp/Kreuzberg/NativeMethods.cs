using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace Kreuzberg;

internal static partial class NativeMethods
{
    private const string LibraryName = "kreuzberg_ffi";

    /// <summary>
    /// Lazy-initialized cache for the native library handle.
    /// Uses ExecutionAndPublication mode to ensure thread-safe, one-time initialization.
    /// This single optimization reduces cold-start time by ~800-900ms (88.7% of cold-start overhead).
    /// </summary>
    private static readonly Lazy<IntPtr> LibraryHandle =
        new(() => LoadNativeLibrary(), LazyThreadSafetyMode.ExecutionAndPublication);

    [ModuleInitializer]
    [SuppressMessage("Usage", "CA2255:The 'ModuleInitializer' attribute should not be used in libraries",
        Justification = "Required for native library resolution before P/Invoke calls. NativeLibrary.SetDllImportResolver must run before any P/Invoke.")]
    internal static void InitResolver()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveLibrary);
    }

    /// <summary>
    /// C-compatible struct for extraction results from the Rust FFI layer.
    /// All string fields are UTF-8 encoded null-terminated pointers.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct CExtractionResult
    {
        // Field order MUST match Rust #[repr(C)] struct (alphabetical by field name)
        /// <summary>JSON array of PDF annotations pointer.</summary>
        public IntPtr AnnotationsJson;
        /// <summary>JSON array of text chunks pointer.</summary>
        public IntPtr ChunksJson;
        /// <summary>JSON array of archive children results pointer.</summary>
        public IntPtr ChildrenJson;
        /// <summary>Extracted text content pointer.</summary>
        public IntPtr Content;
        /// <summary>Document date pointer.</summary>
        public IntPtr Date;
        /// <summary>JSON array of detected languages pointer.</summary>
        public IntPtr DetectedLanguagesJson;
        /// <summary>JSON serialized Djot content pointer.</summary>
        public IntPtr DjotContentJson;
        /// <summary>JSON object of document structure pointer.</summary>
        public IntPtr DocumentJson;
        /// <summary>JSON array of semantic elements pointer.</summary>
        public IntPtr ElementsJson;
        /// <summary>JSON array of extracted keywords pointer.</summary>
        public IntPtr ExtractedKeywordsJson;
        /// <summary>JSON array of extracted images pointer.</summary>
        public IntPtr ImagesJson;
        /// <summary>Detected language pointer.</summary>
        public IntPtr Language;
        /// <summary>JSON object of metadata pointer.</summary>
        public IntPtr MetadataJson;
        /// <summary>MIME type pointer.</summary>
        public IntPtr MimeType;
        /// <summary>JSON array of OCR elements pointer.</summary>
        public IntPtr OcrElementsJson;
        /// <summary>JSON object of page structure pointer.</summary>
        public IntPtr PageStructureJson;
        /// <summary>JSON array of per-page content pointer.</summary>
        public IntPtr PagesJson;
        /// <summary>JSON array of processing warnings pointer.</summary>
        public IntPtr ProcessingWarningsJson;
        /// <summary>JSON quality score pointer.</summary>
        public IntPtr QualityScoreJson;
        /// <summary>Document subject pointer.</summary>
        public IntPtr Subject;
        /// <summary>JSON array of extracted tables pointer.</summary>
        public IntPtr TablesJson;
        /// <summary>JSON array of extracted URIs pointer.</summary>
        public IntPtr UrisJson;

        /// <summary>Whether extraction succeeded.</summary>
        [MarshalAs(UnmanagedType.I1)]
        public bool Success;
    }

    /// <summary>
    /// C-compatible struct for batch extraction results.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct CBatchResult
    {
        // Field order MUST match Rust #[repr(C)] struct: count, results, success
        /// <summary>Number of results in the array.</summary>
        public UIntPtr Count;
        /// <summary>Pointer to array of CExtractionResult structs.</summary>
        public IntPtr Results;

        /// <summary>Whether the batch operation succeeded.</summary>
        [MarshalAs(UnmanagedType.I1)]
        public bool Success;
    }

    /// <summary>
    /// C-compatible struct for document bytes with MIME type.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct CBytesWithMime
    {
        /// <summary>Pointer to raw document bytes.</summary>
        public IntPtr Data;
        /// <summary>Length of document bytes.</summary>
        public UIntPtr DataLen;
        /// <summary>MIME type string pointer.</summary>
        public IntPtr MimeType;
    }

    /// <summary>
    /// Callback delegate for custom OCR backends.
    /// </summary>
    /// <param name="imageBytes">Pointer to raw image bytes.</param>
    /// <param name="imageLength">Length of image bytes.</param>
    /// <param name="configJson">Pointer to OCR config JSON string.</param>
    /// <returns>Pointer to result JSON string allocated by callee.</returns>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr OcrBackendCallback(IntPtr imageBytes, UIntPtr imageLength, IntPtr configJson);

    /// <summary>
    /// Callback delegate for custom post-processors.
    /// </summary>
    /// <param name="resultJson">Pointer to extraction result JSON string.</param>
    /// <returns>Pointer to modified result JSON string allocated by callee.</returns>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr PostProcessorCallback(IntPtr resultJson);

    /// <summary>
    /// Callback delegate for custom validators.
    /// </summary>
    /// <param name="resultJson">Pointer to extraction result JSON string.</param>
    /// <returns>Pointer to error message if validation fails (null if successful), allocated by callee.</returns>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr ValidatorCallback(IntPtr resultJson);

    /// <summary>
    /// Detects MIME type from raw document bytes.
    /// </summary>
    /// <param name="data">Pointer to document bytes.</param>
    /// <param name="dataLen">Length of document bytes.</param>
    /// <returns>Pointer to MIME type string (null if detection fails).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_detect_mime_type_from_bytes", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr DetectMimeTypeFromBytes(IntPtr data, UIntPtr dataLen);

    /// <summary>
    /// Detects MIME type from a file path.
    /// </summary>
    /// <param name="filePath">Pointer to UTF-8 file path string.</param>
    /// <returns>Pointer to MIME type string (null if detection fails or file not found).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_detect_mime_type_from_path", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr DetectMimeTypeFromPath(IntPtr filePath);

    /// <summary>
    /// Gets file extensions for a given MIME type.
    /// </summary>
    /// <param name="mimeType">Pointer to MIME type string.</param>
    /// <returns>Pointer to comma-separated extensions string (null if not found).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_get_extensions_for_mime", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr GetExtensionsForMime(IntPtr mimeType);

    /// <summary>
    /// Discovers and returns default extraction configuration.
    /// </summary>
    /// <returns>Pointer to default config JSON string.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_config_discover", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ConfigDiscover();

    /// <summary>
    /// Extracts content from a file synchronously without custom configuration.
    /// </summary>
    /// <param name="filePath">Pointer to UTF-8 file path string.</param>
    /// <returns>Pointer to CExtractionResult struct (must be freed with FreeResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_file_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractFileSync(IntPtr filePath);

    /// <summary>
    /// Extracts content from a file synchronously with custom configuration.
    /// </summary>
    /// <param name="filePath">Pointer to UTF-8 file path string.</param>
    /// <param name="configJson">Pointer to extraction config JSON string.</param>
    /// <returns>Pointer to CExtractionResult struct (must be freed with FreeResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_file_sync_with_config", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractFileSyncWithConfig(IntPtr filePath, IntPtr configJson);

    /// <summary>
    /// Extracts content from document bytes synchronously without custom configuration.
    /// </summary>
    /// <param name="data">Pointer to document bytes.</param>
    /// <param name="dataLen">Length of document bytes.</param>
    /// <param name="mimeType">Pointer to MIME type string.</param>
    /// <returns>Pointer to CExtractionResult struct (must be freed with FreeResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_bytes_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractBytesSync(IntPtr data, UIntPtr dataLen, IntPtr mimeType);

    /// <summary>
    /// Extracts content from document bytes synchronously with custom configuration.
    /// </summary>
    /// <param name="data">Pointer to document bytes.</param>
    /// <param name="dataLen">Length of document bytes.</param>
    /// <param name="mimeType">Pointer to MIME type string.</param>
    /// <param name="configJson">Pointer to extraction config JSON string.</param>
    /// <returns>Pointer to CExtractionResult struct (must be freed with FreeResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_bytes_sync_with_config", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractBytesSyncWithConfig(IntPtr data, UIntPtr dataLen, IntPtr mimeType, IntPtr configJson);

    /// <summary>
    /// Batch extracts multiple files synchronously with optional per-file config overrides.
    /// </summary>
    /// <param name="filePaths">Pointer to array of UTF-8 file path strings.</param>
    /// <param name="fileConfigJsons">Pointer to array of nullable config JSON strings (IntPtr.Zero for no per-file configs).</param>
    /// <param name="count">Number of file paths.</param>
    /// <param name="configJson">Pointer to batch-level extraction config JSON string.</param>
    /// <returns>Pointer to CBatchResult struct (must be freed with FreeBatchResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_batch_extract_files_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr BatchExtractFilesSync(IntPtr filePaths, IntPtr fileConfigJsons, UIntPtr count, IntPtr configJson);

    /// <summary>
    /// Batch extracts multiple documents from bytes synchronously with optional per-file config overrides.
    /// </summary>
    /// <param name="items">Pointer to array of CBytesWithMime structs.</param>
    /// <param name="fileConfigJsons">Pointer to array of nullable config JSON strings (IntPtr.Zero for no per-file configs).</param>
    /// <param name="count">Number of items.</param>
    /// <param name="configJson">Pointer to batch-level extraction config JSON string.</param>
    /// <returns>Pointer to CBatchResult struct (must be freed with FreeBatchResult).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_batch_extract_bytes_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr BatchExtractBytesSync(IntPtr items, IntPtr fileConfigJsons, UIntPtr count, IntPtr configJson);

    /// <summary>
    /// Loads extraction configuration from a JSON file.
    /// </summary>
    /// <param name="filePath">Pointer to UTF-8 file path string.</param>
    /// <returns>Pointer to config JSON string (null if file not found or parse error).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_load_extraction_config_from_file", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr LoadExtractionConfigFromFile(IntPtr filePath);

    /// <summary>
    /// Frees a batch result struct and its contents.
    /// </summary>
    /// <param name="batchResult">Pointer to CBatchResult struct.</param>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_batch_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeBatchResult(IntPtr batchResult);

    /// <summary>
    /// Frees a string allocated by the native library.
    /// </summary>
    /// <param name="ptr">Pointer to native string.</param>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_string", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeString(IntPtr ptr);

    /// <summary>
    /// Creates a copy of a native string.
    /// The returned string must be freed with FreeString.
    /// </summary>
    /// <param name="ptr">Pointer to native string to clone.</param>
    /// <returns>Pointer to cloned string.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_clone_string", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr CloneString(IntPtr ptr);

    /// <summary>
    /// Frees an extraction result struct and its contents.
    /// </summary>
    /// <param name="result">Pointer to CExtractionResult struct.</param>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeResult(IntPtr result);

    /// <summary>
    /// Gets the last error message from the native library (thread-safe).
    /// </summary>
    /// <returns>Pointer to error message string (null if no error).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_last_error", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr LastError();

    /// <summary>
    /// Gets the Kreuzberg library version string.
    /// </summary>
    /// <returns>Pointer to version string (e.g., "4.0.0").</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_version", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr Version();

    /// <summary>
    /// Registers a custom OCR backend implementation.
    /// </summary>
    /// <param name="name">Pointer to backend name UTF-8 string.</param>
    /// <param name="callback">Callback function to invoke for OCR processing.</param>
    /// <returns>True if registration succeeded, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_ocr_backend", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterOcrBackend(IntPtr name, OcrBackendCallback callback);

    /// <summary>
    /// Registers a custom post-processor implementation.
    /// </summary>
    /// <param name="name">Pointer to processor name UTF-8 string.</param>
    /// <param name="callback">Callback function to invoke for post-processing.</param>
    /// <param name="priority">Execution priority (higher values run first).</param>
    /// <returns>True if registration succeeded, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_post_processor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterPostProcessor(IntPtr name, PostProcessorCallback callback, int priority);

    /// <summary>
    /// Unregisters a previously registered OCR backend by name.
    /// </summary>
    /// <param name="name">Pointer to backend name UTF-8 string.</param>
    /// <returns>True if unregistration succeeded, false if backend not found.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_ocr_backend", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterOcrBackend(IntPtr name);

    /// <summary>
    /// Gets a list of all registered OCR backends as a JSON array string.
    /// </summary>
    /// <returns>Pointer to JSON array string of backend names.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_ocr_backends", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListOcrBackends();

    /// <summary>
    /// Clears all registered OCR backends.
    /// </summary>
    /// <returns>True if all cleared successfully, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_ocr_backends", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearOcrBackends();

    /// <summary>
    /// Unregisters a previously registered post-processor by name.
    /// </summary>
    /// <param name="name">Pointer to processor name UTF-8 string.</param>
    /// <returns>True if unregistration succeeded, false if processor not found.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_post_processor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterPostProcessor(IntPtr name);

    /// <summary>
    /// Clears all registered post-processors.
    /// </summary>
    /// <returns>True if all cleared successfully, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_post_processors", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearPostProcessors();

    /// <summary>
    /// Gets a list of all registered post-processors as a JSON array string.
    /// </summary>
    /// <returns>Pointer to JSON array string of processor names with priorities.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_post_processors", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListPostProcessors();

    /// <summary>
    /// Registers a custom validator implementation.
    /// </summary>
    /// <param name="name">Pointer to validator name UTF-8 string.</param>
    /// <param name="callback">Callback function to invoke for validation.</param>
    /// <param name="priority">Execution priority (higher values run first).</param>
    /// <returns>True if registration succeeded, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterValidator(IntPtr name, ValidatorCallback callback, int priority);

    /// <summary>
    /// Unregisters a previously registered validator by name.
    /// </summary>
    /// <param name="name">Pointer to validator name UTF-8 string.</param>
    /// <returns>True if unregistration succeeded, false if validator not found.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterValidator(IntPtr name);

    /// <summary>
    /// Clears all registered validators.
    /// </summary>
    /// <returns>True if all cleared successfully, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_validators", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearValidators();

    /// <summary>
    /// Gets a list of all registered validators as a JSON array string.
    /// </summary>
    /// <returns>Pointer to JSON array string of validator names with priorities.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_validators", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListValidators();

    /// <summary>
    /// Gets a list of all available document extractors as a JSON array string.
    /// </summary>
    /// <returns>Pointer to JSON array string of extractor names.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_document_extractors", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListDocumentExtractors();

    /// <summary>
    /// Clears all registered document extractors.
    /// </summary>
    /// <returns>True if all cleared successfully, false otherwise.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_document_extractors", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearDocumentExtractors();

    /// <summary>
    /// Unregisters a previously registered document extractor by name.
    /// </summary>
    /// <param name="name">Pointer to extractor name UTF-8 string.</param>
    /// <returns>True if unregistration succeeded, false if extractor not found.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_document_extractor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterDocumentExtractor(IntPtr name);

    /// <summary>
    /// Gets a list of all available embedding presets as a JSON array string.
    /// </summary>
    /// <returns>Pointer to JSON array string of preset definitions.</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_embedding_presets", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListEmbeddingPresets();

    /// <summary>
    /// Gets a specific embedding preset definition by name.
    /// </summary>
    /// <param name="name">Pointer to preset name UTF-8 string.</param>
    /// <returns>Pointer to preset definition JSON string (null if not found).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_get_embedding_preset", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr GetEmbeddingPreset(IntPtr name);

    /// <summary>
    /// C-compatible struct for a single rendered page image (PNG bytes).
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct CPageImage
    {
        /// <summary>Pointer to PNG data.</summary>
        public IntPtr Data;
        /// <summary>Length of PNG data in bytes.</summary>
        public UIntPtr Len;
    }

    /// <summary>
    /// Renders a single PDF page as a PNG image.
    /// </summary>
    /// <param name="filePath">Pointer to UTF-8 file path string.</param>
    /// <param name="pageIndex">Zero-based page index.</param>
    /// <param name="dpi">Rendering resolution in DPI.</param>
    /// <returns>Pointer to CPageImage struct (null on failure, check LastError).</returns>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_render_pdf_page", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr RenderPdfPage(IntPtr filePath, UIntPtr pageIndex, int dpi);

    /// <summary>
    /// Frees a single page result returned by RenderPdfPage.
    /// </summary>
    /// <param name="page">Pointer to CPageImage struct.</param>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_render_page_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeRenderPageResult(IntPtr page);

    /// <summary>
    /// Creates a new PDF page iterator from a file path.
    /// </summary>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_pdf_page_iterator_new", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr PdfPageIteratorNew(IntPtr filePath, int dpi);

    /// <summary>
    /// Advances the iterator and returns the next rendered page (CPageIterResult).
    /// Returns NULL when iteration is complete. Check LastError() to distinguish
    /// exhaustion (no error) from failure (error set).
    /// </summary>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_pdf_page_iterator_next", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr PdfPageIteratorNext(IntPtr iter);

    /// <summary>
    /// Frees a single iterator result returned by PdfPageIteratorNext.
    /// </summary>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_pdf_page_iterator_free_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void PdfPageIteratorFreeResult(IntPtr result);

    /// <summary>
    /// Returns the total number of pages in the PDF.
    /// </summary>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_pdf_page_iterator_page_count", CallingConvention = CallingConvention.Cdecl)]
    internal static extern UIntPtr PdfPageIteratorPageCount(IntPtr iter);

    /// <summary>
    /// Frees a PDF page iterator.
    /// </summary>
    [DllImport(LibraryName, EntryPoint = "kreuzberg_pdf_page_iterator_free", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void PdfPageIteratorFree(IntPtr iter);

    /// <summary>
    /// Native CPageIterResult layout: usize page_index, *mut u8 data, usize len.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct CPageIterResult
    {
        public UIntPtr PageIndex;
        public IntPtr Data;
        public UIntPtr Len;
    }

    private static IntPtr ResolveLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, LibraryName, StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        var effectiveSearchPath = searchPath ?? DllImportSearchPath.AssemblyDirectory;
        if (NativeLibrary.TryLoad(libraryName, assembly, effectiveSearchPath, out var defaultHandle))
        {
            return defaultHandle;
        }

        return LibraryHandle.Value;
    }

    private static IntPtr LoadNativeLibrary()
    {
        var fileName = GetLibraryFileName();
        var debug = Environment.GetEnvironmentVariable("KREUZBERG_BENCHMARK_DEBUG") == "true";
        var probePaths = GetProbePaths(fileName).ToList();

        if (debug)
        {
            System.Console.Error.WriteLine($"[DEBUG] Looking for native library: {fileName}");
            System.Console.Error.WriteLine($"[DEBUG] Probe paths:");
            foreach (var path in probePaths)
            {
                var exists = System.IO.File.Exists(path);
                System.Console.Error.WriteLine($"[DEBUG]   {path} (exists: {exists})");
            }
        }

        foreach (var path in probePaths)
        {
            TryPreloadSiblingNativeDependencies(Path.GetDirectoryName(path), debug);
            if (NativeLibrary.TryLoad(path, out var handle))
            {
                if (debug)
                {
                    System.Console.Error.WriteLine($"[DEBUG] Successfully loaded native library from: {path}");
                }
                return handle;
            }
        }

        var pathsStr = string.Join(", ", probePaths);
        throw new DllNotFoundException($"Unable to locate {fileName}. Checked: {pathsStr}. Set KREUZBERG_FFI_DIR or place the library in target/release.");
    }

    private static IEnumerable<string> GetProbePaths(string fileName)
    {
        var envDir = Environment.GetEnvironmentVariable("KREUZBERG_FFI_DIR");
        if (!string.IsNullOrWhiteSpace(envDir))
        {
            yield return Path.Combine(envDir, fileName);
        }

        yield return Path.Combine(AppContext.BaseDirectory, fileName);

        var rid = GetStableRuntimeIdentifier();
        if (!string.IsNullOrWhiteSpace(rid))
        {
            yield return Path.Combine(AppContext.BaseDirectory, "runtimes", rid!, "native", fileName);
        }

        var cwd = Directory.GetCurrentDirectory();
        yield return Path.Combine(cwd, fileName);

        var cwdRelease = Path.Combine(cwd, "target", "release", fileName);
        if (File.Exists(cwdRelease))
        {
            yield return cwdRelease;
        }

        var cwdDebug = Path.Combine(cwd, "target", "debug", fileName);
        if (File.Exists(cwdDebug))
        {
            yield return cwdDebug;
        }

        string? dir = AppContext.BaseDirectory;
        for (var i = 0; i < 5 && dir != null; i++)
        {
            var release = Path.Combine(dir, "target", "release", fileName);
            if (File.Exists(release))
            {
                yield return release;
            }

            var debugPath = Path.Combine(dir, "target", "debug", fileName);
            if (File.Exists(debugPath))
            {
                yield return debugPath;
            }

            dir = Directory.GetParent(dir)?.FullName;
        }
    }

    private static void TryPreloadSiblingNativeDependencies(string? directory, bool debug)
    {
        if (string.IsNullOrWhiteSpace(directory) || !Directory.Exists(directory))
        {
            return;
        }

        var pdfium = GetPdfiumFileName();
        var candidates = new List<string> { Path.Combine(directory!, pdfium) };

        if (OperatingSystem.IsWindows())
        {
            candidates.AddRange(Directory.EnumerateFiles(directory!, "onnxruntime*.dll"));
        }
        else if (OperatingSystem.IsMacOS())
        {
            candidates.AddRange(Directory.EnumerateFiles(directory!, "libonnxruntime*.dylib"));
        }
        else
        {
            candidates.AddRange(Directory.EnumerateFiles(directory!, "libonnxruntime*.so*"));
        }

        foreach (var candidate in candidates.Distinct(StringComparer.Ordinal))
        {
            if (!File.Exists(candidate))
            {
                continue;
            }

            if (NativeLibrary.TryLoad(candidate, out _))
            {
                if (debug)
                {
                    System.Console.Error.WriteLine($"[DEBUG] Preloaded native dependency: {candidate}");
                }
            }
        }
    }

    private static string? GetStableRuntimeIdentifier()
    {
        var arch = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => null,
        };

        if (arch is null)
        {
            return null;
        }

        if (OperatingSystem.IsWindows())
        {
            return $"win-{arch}";
        }

        if (OperatingSystem.IsMacOS())
        {
            return $"osx-{arch}";
        }

        if (OperatingSystem.IsLinux())
        {
            return $"linux-{arch}";
        }

        return null;
    }

    private static string GetLibraryFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "kreuzberg_ffi.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libkreuzberg_ffi.dylib";
        }

        return "libkreuzberg_ffi.so";
    }

    private static string GetPdfiumFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "pdfium.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libpdfium.dylib";
        }

        return "libpdfium.so";
    }
}
