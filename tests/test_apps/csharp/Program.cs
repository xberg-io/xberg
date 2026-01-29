using System.Text.Json;
using Kreuzberg;

namespace KreuzbergSmokeTest;

internal sealed class Program
{
    private sealed class TestResult
    {
        public bool Success { get; set; }
        public string File { get; set; } = string.Empty;
        public int? TextLength { get; set; }
        public string? Preview { get; set; }
        public bool ForceOcr { get; set; }
        public string? Error { get; set; }
        public string? ErrorType { get; set; }
    }

    // Custom post-processor for testing plugin system
    private sealed class TestPostProcessor : IPostProcessor
    {
        public string Name => "test-uppercase";
        public int Priority => 10;

        public ExtractionResult Process(ExtractionResult result)
        {
            result.Content = result.Content.ToUpperInvariant();
            return result;
        }
    }

    // Custom validator for testing validator system
    private sealed class TestValidator : IValidator
    {
        public string Name => "test-non-empty";
        public int Priority => 5;

        public void Validate(ExtractionResult result)
        {
            if (string.IsNullOrWhiteSpace(result.Content))
            {
                throw new KreuzbergValidationException("Test validator: content cannot be empty");
            }
        }
    }

    // Custom OCR backend for testing OCR backend system
    private sealed class TestOcrBackend : IOcrBackend
    {
        public string Name => "test-dummy-ocr";

        public string Process(ReadOnlySpan<byte> imageData, OcrConfig? config)
        {
            return $"[Test OCR: {imageData.Length} bytes]";
        }
    }

    private static TestResult TestDocument(string filePath, bool forceOcr = false)
    {
        try
        {
            var config = new ExtractionConfig
            {
                ForceOcr = forceOcr
            };

            var result = KreuzbergClient.ExtractFileSync(filePath, config);

            var extractedText = result.Content ?? string.Empty;
            var textPreview = extractedText.Length > 100
                ? extractedText[..100].Replace("\n", " ")
                : extractedText.Replace("\n", " ");

            return new TestResult
            {
                Success = true,
                File = Path.GetFileName(filePath),
                TextLength = extractedText.Length,
                Preview = textPreview,
                ForceOcr = forceOcr
            };
        }
        catch (Exception ex)
        {
            return new TestResult
            {
                Success = false,
                File = Path.GetFileName(filePath),
                Error = ex.Message,
                ErrorType = ex.GetType().Name,
                ForceOcr = forceOcr
            };
        }
    }

    private static int Main()
    {
        var testDocsDir = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "..", "..", "..", "test_documents");
        testDocsDir = Path.GetFullPath(testDocsDir);

        if (!Directory.Exists(testDocsDir))
        {
            Console.WriteLine($"ERROR: test_documents directory not found at {testDocsDir}");
            return 1;
        }

        var version = KreuzbergClient.GetVersion();
        Console.WriteLine($"Starting kreuzberg {version} test suite");
        Console.WriteLine($"Test documents directory: {testDocsDir}");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        // Run all test categories
        allPassed &= RunVersionTests();
        allPassed &= RunMimeDetectionTests(testDocsDir);
        allPassed &= RunErrorHandlingTests();
        allPassed &= RunPluginSystemTests();
        allPassed &= RunAsyncTests(testDocsDir);
        allPassed &= RunBatchOperationTests(testDocsDir);
        allPassed &= RunConfigurationTests();
        allPassed &= RunResultDeserializationTests(testDocsDir);
        allPassed &= RunEdgeCaseTests();
        allPassed &= RunMissingAPITests(testDocsDir);
        allPassed &= RunOriginalTests(testDocsDir);

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("FINAL TEST SUMMARY");
        Console.WriteLine(new string('-', 80));

        if (allPassed)
        {
            Console.WriteLine("\nAll test categories passed!");
            return 0;
        }
        else
        {
            Console.WriteLine("\nSome test categories failed!");
            return 1;
        }
    }

    private static bool RunVersionTests()
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Version Tests");
        Console.WriteLine(new string('-', 80));

        try
        {
            var version = KreuzbergClient.GetVersion();
            Console.WriteLine($"TEST  GetVersion()                         OK   (version: {version})");
            return true;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"TEST  GetVersion()                         FAIL");
            Console.WriteLine($"      Error: {ex.Message}");
            return false;
        }
    }

    private static bool RunMimeDetectionTests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("MIME Type Detection Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;
        var testFiles = new[] { "tiny.pdf", "lorem_ipsum.docx", "stanley_cups.xlsx" };

        // Test DetectMimeTypeFromPath
        foreach (var fileName in testFiles)
        {
            var filePath = Path.Combine(testDocsDir, fileName);
            if (!File.Exists(filePath))
                continue;

            try
            {
                Console.Write($"TEST  DetectMimeTypeFromPath({fileName})   ");
                var mimeType = KreuzbergClient.DetectMimeTypeFromPath(filePath);
                Console.WriteLine($"OK   (MIME: {mimeType})");
            }
            catch (Exception ex)
            {
                Console.WriteLine("FAIL");
                Console.WriteLine($"      Error: {ex.Message}");
                allPassed = false;
            }
        }

        // Test DetectMimeType from bytes
        var pdfPath = Path.Combine(testDocsDir, "tiny.pdf");
        if (File.Exists(pdfPath))
        {
            try
            {
                Console.Write($"TEST  DetectMimeType(bytes)                ");
                var bytes = File.ReadAllBytes(pdfPath);
                var mimeType = KreuzbergClient.DetectMimeType(bytes);
                Console.WriteLine($"OK   (MIME: {mimeType})");
            }
            catch (Exception ex)
            {
                Console.WriteLine("FAIL");
                Console.WriteLine($"      Error: {ex.Message}");
                allPassed = false;
            }
        }

        // Test GetExtensionsForMime
        try
        {
            Console.Write($"TEST  GetExtensionsForMime(application/pdf) ");
            var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
            Console.WriteLine($"OK   (extensions: {string.Join(", ", extensions)})");
        }
        catch (Exception ex)
        {
            Console.WriteLine("FAIL");
            Console.WriteLine($"      Error: {ex.Message}");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunErrorHandlingTests()
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Error Handling Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        // Test validation errors - empty path
        try
        {
            Console.Write($"TEST  ExtractFileSync(empty path)          ");
            KreuzbergClient.ExtractFileSync("");
            Console.WriteLine("FAIL (should have thrown exception)");
            allPassed = false;
        }
        catch (ArgumentException)
        {
            Console.WriteLine("OK   (caught ArgumentException)");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL (wrong exception: {ex.GetType().Name})");
            allPassed = false;
        }

        // Test non-existent file
        try
        {
            Console.Write($"TEST  ExtractFileSync(missing file)        ");
            KreuzbergClient.ExtractFileSync("/nonexistent/path/file.pdf");
            Console.WriteLine("FAIL (should have thrown exception)");
            allPassed = false;
        }
        catch (KreuzbergIOException)
        {
            Console.WriteLine("OK   (caught KreuzbergIOException)");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL (caught {ex.GetType().Name}: {ex.Message})");
            allPassed = false;
        }

        // Test empty bytes detection
        try
        {
            Console.Write($"TEST  DetectMimeType(empty bytes)          ");
            KreuzbergClient.DetectMimeType(Array.Empty<byte>());
            Console.WriteLine("FAIL (should have thrown exception)");
            allPassed = false;
        }
        catch (KreuzbergValidationException)
        {
            Console.WriteLine("OK   (caught KreuzbergValidationException)");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL (caught {ex.GetType().Name})");
            allPassed = false;
        }

        // Test empty MIME type
        try
        {
            Console.Write($"TEST  GetExtensionsForMime(empty)          ");
            KreuzbergClient.GetExtensionsForMime("");
            Console.WriteLine("FAIL (should have thrown exception)");
            allPassed = false;
        }
        catch (KreuzbergValidationException)
        {
            Console.WriteLine("OK   (caught KreuzbergValidationException)");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL (caught {ex.GetType().Name})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunPluginSystemTests()
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Plugin System Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        try
        {
            // Test post-processor registration
            Console.Write($"TEST  RegisterPostProcessor()              ");
            KreuzbergClient.ClearPostProcessors();
            KreuzbergClient.RegisterPostProcessor(new TestPostProcessor());
            var processors = KreuzbergClient.ListPostProcessors();
            if (processors.Contains("test-uppercase"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (post-processor not registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ListPostProcessors()                 ");
            var processors = KreuzbergClient.ListPostProcessors();
            Console.WriteLine($"OK   (count: {processors.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  UnregisterPostProcessor()            ");
            KreuzbergClient.UnregisterPostProcessor("test-uppercase");
            var processors = KreuzbergClient.ListPostProcessors();
            if (!processors.Contains("test-uppercase"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (post-processor still registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test validator registration
        try
        {
            Console.Write($"TEST  RegisterValidator()                  ");
            KreuzbergClient.ClearValidators();
            KreuzbergClient.RegisterValidator(new TestValidator());
            var validators = KreuzbergClient.ListValidators();
            if (validators.Contains("test-non-empty"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (validator not registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ListValidators()                     ");
            var validators = KreuzbergClient.ListValidators();
            Console.WriteLine($"OK   (count: {validators.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  UnregisterValidator()                ");
            KreuzbergClient.UnregisterValidator("test-non-empty");
            var validators = KreuzbergClient.ListValidators();
            if (!validators.Contains("test-non-empty"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (validator still registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test OCR backend registration
        try
        {
            Console.Write($"TEST  RegisterOcrBackend()                 ");
            KreuzbergClient.ClearOcrBackends();
            KreuzbergClient.RegisterOcrBackend(new TestOcrBackend());
            var backends = KreuzbergClient.ListOcrBackends();
            if (backends.Contains("test-dummy-ocr"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (OCR backend not registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ListOcrBackends()                    ");
            var backends = KreuzbergClient.ListOcrBackends();
            Console.WriteLine($"OK   (count: {backends.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  UnregisterOcrBackend()               ");
            KreuzbergClient.UnregisterOcrBackend("test-dummy-ocr");
            var backends = KreuzbergClient.ListOcrBackends();
            if (!backends.Contains("test-dummy-ocr"))
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (OCR backend still registered)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test document extractor listing
        try
        {
            Console.Write($"TEST  ListDocumentExtractors()             ");
            var extractors = KreuzbergClient.ListDocumentExtractors();
            Console.WriteLine($"OK   (count: {extractors.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunAsyncTests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Async/Await Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;
        var pdfPath = Path.Combine(testDocsDir, "tiny.pdf");

        if (!File.Exists(pdfPath))
        {
            Console.WriteLine("SKIP  tiny.pdf not found");
            return true;
        }

        try
        {
            Console.Write($"TEST  ExtractFileAsync()                   ");
            var task = KreuzbergClient.ExtractFileAsync(pdfPath);
            var result = task.Result;
            if (result != null && result.Success)
            {
                Console.WriteLine($"OK   (content: {result.Content?.Length ?? 0} chars)");
            }
            else
            {
                Console.WriteLine("FAIL (extraction failed)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractBytesAsync()                  ");
            var bytes = File.ReadAllBytes(pdfPath);
            var task = KreuzbergClient.ExtractBytesAsync(bytes, "application/pdf");
            var result = task.Result;
            if (result != null && result.Success)
            {
                Console.WriteLine($"OK   (content: {result.Content?.Length ?? 0} chars)");
            }
            else
            {
                Console.WriteLine("FAIL (extraction failed)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test cancellation token
        try
        {
            Console.Write($"TEST  ExtractFileAsync(cancellation)       ");
            var cts = new System.Threading.CancellationTokenSource();
            // Cancel immediately without waiting for actual completion
            cts.Cancel();
            try
            {
                var task = KreuzbergClient.ExtractFileAsync(pdfPath, null, cts.Token);
                var result = task.Result;
                Console.WriteLine("FAIL  (cancellation should have triggered)");
                allPassed = false;
            }
            catch (AggregateException ae) when (ae.InnerException is OperationCanceledException)
            {
                Console.WriteLine("OK   (operation canceled)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunBatchOperationTests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Batch Operation Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;
        var pdfPath = Path.Combine(testDocsDir, "tiny.pdf");
        var docxPath = Path.Combine(testDocsDir, "lorem_ipsum.docx");

        // Test BatchExtractFilesSync
        try
        {
            Console.Write($"TEST  BatchExtractFilesSync()              ");
            var existingFiles = new[] { pdfPath, docxPath }.Where(f => File.Exists(f)).ToList();
            if (existingFiles.Count > 0)
            {
                var results = KreuzbergClient.BatchExtractFilesSync(existingFiles);
                Console.WriteLine($"OK   (count: {results.Count})");
            }
            else
            {
                Console.WriteLine("SKIP  (no test files found)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test BatchExtractBytesSync
        try
        {
            Console.Write($"TEST  BatchExtractBytesSync()              ");
            if (File.Exists(pdfPath))
            {
                var items = new[]
                {
                    new BytesWithMime(File.ReadAllBytes(pdfPath), "application/pdf")
                };
                var results = KreuzbergClient.BatchExtractBytesSync(items);
                Console.WriteLine($"OK   (count: {results.Count})");
            }
            else
            {
                Console.WriteLine("SKIP  (tiny.pdf not found)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test BatchExtractFilesAsync
        try
        {
            Console.Write($"TEST  BatchExtractFilesAsync()             ");
            var existingFiles = new[] { pdfPath, docxPath }.Where(f => File.Exists(f)).ToList();
            if (existingFiles.Count > 0)
            {
                var task = KreuzbergClient.BatchExtractFilesAsync(existingFiles);
                var results = task.Result;
                Console.WriteLine($"OK   (count: {results.Count})");
            }
            else
            {
                Console.WriteLine("SKIP  (no test files found)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test BatchExtractBytesAsync
        try
        {
            Console.Write($"TEST  BatchExtractBytesAsync()             ");
            if (File.Exists(pdfPath))
            {
                var items = new[]
                {
                    new BytesWithMime(File.ReadAllBytes(pdfPath), "application/pdf")
                };
                var task = KreuzbergClient.BatchExtractBytesAsync(items);
                var results = task.Result;
                Console.WriteLine($"OK   (count: {results.Count})");
            }
            else
            {
                Console.WriteLine("SKIP  (tiny.pdf not found)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunConfigurationTests()
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Configuration Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        // Test ExtractionConfig properties
        try
        {
            Console.Write($"TEST  ExtractionConfig.UseCache            ");
            var config = new ExtractionConfig { UseCache = true };
            if (config.UseCache == true)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionConfig.EnableQualityProc  ");
            var config = new ExtractionConfig { EnableQualityProcessing = true };
            if (config.EnableQualityProcessing == true)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionConfig.MaxConcurrentExtr   ");
            var config = new ExtractionConfig { MaxConcurrentExtractions = 4 };
            if (config.MaxConcurrentExtractions == 4)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test nested config classes
        try
        {
            Console.Write($"TEST  OcrConfig properties                 ");
            var ocrConfig = new OcrConfig
            {
                Backend = "tesseract",
                Language = "eng"
            };
            if (ocrConfig.Backend == "tesseract" && ocrConfig.Language == "eng")
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ChunkingConfig properties            ");
            var chunkConfig = new ChunkingConfig
            {
                MaxChars = 1024,
                MaxOverlap = 256
            };
            if (chunkConfig.MaxChars == 1024 && chunkConfig.MaxOverlap == 256)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ImageExtractionConfig properties     ");
            var imgConfig = new ImageExtractionConfig
            {
                ExtractImages = true
            };
            if (imgConfig.ExtractImages == true)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ImagePreprocessingConfig properties  ");
            var prepConfig = new ImagePreprocessingConfig
            {
                TargetDpi = 300,
                AutoRotate = true
            };
            if (prepConfig.TargetDpi == 300 && prepConfig.AutoRotate == true)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  EmbeddingConfig properties           ");
            var embedConfig = new EmbeddingConfig
            {
                Model = "default",
                BatchSize = 32
            };
            if (embedConfig.Model == "default" && embedConfig.BatchSize == 32)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunResultDeserializationTests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Result Deserialization Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;
        var pdfPath = Path.Combine(testDocsDir, "tiny.pdf");

        if (!File.Exists(pdfPath))
        {
            Console.WriteLine("SKIP  tiny.pdf not found");
            return true;
        }

        try
        {
            Console.Write($"TEST  ExtractionResult.Content             ");
            var result = KreuzbergClient.ExtractFileSync(pdfPath);
            if (!string.IsNullOrEmpty(result.Content))
            {
                Console.WriteLine($"OK   ({result.Content.Length} chars)");
            }
            else
            {
                Console.WriteLine("FAIL (content is empty)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionResult.MimeType            ");
            var result = KreuzbergClient.ExtractFileSync(pdfPath);
            if (!string.IsNullOrEmpty(result.MimeType))
            {
                Console.WriteLine($"OK   ({result.MimeType})");
            }
            else
            {
                Console.WriteLine("FAIL (MIME type is empty)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionResult.Metadata            ");
            var result = KreuzbergClient.ExtractFileSync(pdfPath);
            if (result.Metadata != null)
            {
                Console.WriteLine($"OK   (type: {result.Metadata.FormatType})");
            }
            else
            {
                Console.WriteLine("FAIL (metadata is null)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionResult.Tables              ");
            var result = KreuzbergClient.ExtractFileSync(pdfPath);
            Console.WriteLine($"OK   (count: {result.Tables.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  ExtractionResult.Success             ");
            var result = KreuzbergClient.ExtractFileSync(pdfPath);
            Console.WriteLine($"OK   (success: {result.Success})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunEdgeCaseTests()
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Edge Case Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        // Test empty batch
        try
        {
            Console.Write($"TEST  BatchExtractFilesSync(empty list)    ");
            var results = KreuzbergClient.BatchExtractFilesSync(Array.Empty<string>());
            if (results.Count == 0)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (expected empty result)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  BatchExtractBytesSync(empty list)    ");
            var results = KreuzbergClient.BatchExtractBytesSync(Array.Empty<BytesWithMime>());
            if (results.Count == 0)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (expected empty result)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test null checks
        try
        {
            Console.Write($"TEST  BatchExtractFilesSync(null)          ");
            try
            {
                KreuzbergClient.BatchExtractFilesSync(null!);
                Console.WriteLine("FAIL (should throw ArgumentNullException)");
                allPassed = false;
            }
            catch (ArgumentNullException)
            {
                Console.WriteLine("OK   (caught ArgumentNullException)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        try
        {
            Console.Write($"TEST  RegisterPostProcessor(null)          ");
            try
            {
                KreuzbergClient.RegisterPostProcessor(null!);
                Console.WriteLine("FAIL (should throw ArgumentNullException)");
                allPassed = false;
            }
            catch (ArgumentNullException)
            {
                Console.WriteLine("OK   (caught ArgumentNullException)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunMissingAPITests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Additional API Coverage Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;

        // Test ClearPostProcessors explicitly
        try
        {
            Console.Write($"TEST  ClearPostProcessors()                ");
            KreuzbergClient.RegisterPostProcessor(new TestPostProcessor());
            KreuzbergClient.ClearPostProcessors();
            var processors = KreuzbergClient.ListPostProcessors();
            if (processors.Count == 0)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (processors not cleared)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test ClearValidators explicitly
        try
        {
            Console.Write($"TEST  ClearValidators()                    ");
            KreuzbergClient.RegisterValidator(new TestValidator());
            KreuzbergClient.ClearValidators();
            var validators = KreuzbergClient.ListValidators();
            if (validators.Count == 0)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (validators not cleared)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test ClearOcrBackends explicitly
        try
        {
            Console.Write($"TEST  ClearOcrBackends()                   ");
            KreuzbergClient.RegisterOcrBackend(new TestOcrBackend());
            KreuzbergClient.ClearOcrBackends();
            var backends = KreuzbergClient.ListOcrBackends();
            if (backends.Count == 0)
            {
                Console.WriteLine("OK");
            }
            else
            {
                Console.WriteLine("FAIL (backends not cleared)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test ClearDocumentExtractors
        try
        {
            Console.Write($"TEST  ClearDocumentExtractors()            ");
            KreuzbergClient.ClearDocumentExtractors();
            Console.WriteLine("OK");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test UnregisterDocumentExtractor with validation
        try
        {
            Console.Write($"TEST  UnregisterDocumentExtractor()        ");
            var extractors = KreuzbergClient.ListDocumentExtractors();

            if (extractors.Count > 0)
            {
                // Test unregistering an actual extractor
                var extractorName = extractors[0];
                try
                {
                    KreuzbergClient.UnregisterDocumentExtractor(extractorName);
                    var remaining = KreuzbergClient.ListDocumentExtractors();
                    if (!remaining.Contains(extractorName))
                    {
                        Console.WriteLine("OK   (extractor unregistered successfully)");
                    }
                    else
                    {
                        Console.WriteLine("FAIL (extractor still in list after unregister)");
                        allPassed = false;
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"FAIL ({ex.Message})");
                    allPassed = false;
                }
            }
            else
            {
                // No extractors to test, test that it validates input properly
                try
                {
                    KreuzbergClient.UnregisterDocumentExtractor("nonexistent-extractor-xyz");
                    Console.WriteLine("OK   (handles non-existent extractor)");
                }
                catch (KreuzbergException)
                {
                    Console.WriteLine("OK   (raises KreuzbergException for missing extractor)");
                }
                catch (ArgumentException)
                {
                    Console.WriteLine("OK   (raises ArgumentException for missing extractor)");
                }
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test DiscoverExtractionConfig
        try
        {
            Console.Write($"TEST  DiscoverExtractionConfig()           ");
            var config = KreuzbergClient.DiscoverExtractionConfig();
            if (config == null)
            {
                Console.WriteLine("OK   (null - no config file in current directory)");
            }
            else
            {
                Console.WriteLine("OK   (found config file)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test LoadExtractionConfigFromFile with missing file
        try
        {
            Console.Write($"TEST  LoadExtractionConfigFromFile()       ");
            try
            {
                KreuzbergClient.LoadExtractionConfigFromFile("/nonexistent/config.toml");
                Console.WriteLine("FAIL (should throw exception for missing file)");
                allPassed = false;
            }
            catch (KreuzbergValidationException)
            {
                Console.WriteLine("OK   (throws exception for missing file)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test ListEmbeddingPresets
        try
        {
            Console.Write($"TEST  ListEmbeddingPresets()               ");
            var presets = KreuzbergClient.ListEmbeddingPresets();
            Console.WriteLine($"OK   (count: {presets.Count})");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test GetEmbeddingPreset with valid name (or get first if exists)
        try
        {
            Console.Write($"TEST  GetEmbeddingPreset()                 ");
            var presets = KreuzbergClient.ListEmbeddingPresets();
            if (presets.Count > 0)
            {
                var preset = KreuzbergClient.GetEmbeddingPreset(presets[0]);
                if (preset != null)
                {
                    Console.WriteLine($"OK   (preset: {preset.ModelName})");
                }
                else
                {
                    Console.WriteLine("FAIL (preset not found)");
                    allPassed = false;
                }
            }
            else
            {
                Console.WriteLine("SKIP  (no presets available)");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        // Test GetEmbeddingPreset with invalid name
        try
        {
            Console.Write($"TEST  GetEmbeddingPreset(invalid)          ");
            var preset = KreuzbergClient.GetEmbeddingPreset("nonexistent-preset");
            if (preset == null)
            {
                Console.WriteLine("OK   (returns null for invalid preset)");
            }
            else
            {
                Console.WriteLine("FAIL (should return null)");
                allPassed = false;
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAIL ({ex.Message})");
            allPassed = false;
        }

        return allPassed;
    }

    private static bool RunOriginalTests(string testDocsDir)
    {
        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Original Extraction Tests");
        Console.WriteLine(new string('-', 80));

        var allPassed = true;
        var results = new List<TestResult>();

        var documents = new (string Name, string Type)[]
        {
            ("tiny.pdf", "PDF"),
            ("lorem_ipsum.docx", "DOCX"),
            ("stanley_cups.xlsx", "XLSX"),
            ("ocr_image.jpg", "JPG Image"),
            ("test_hello_world.png", "PNG Image")
        };

        foreach (var (docName, docType) in documents)
        {
            var docPath = Path.Combine(testDocsDir, docName);

            if (!File.Exists(docPath))
            {
                Console.WriteLine($"SKIP  {docType,-15} {docName,-30} - File not found");
                continue;
            }

            Console.Write($"TEST  {docType,-15} {docName,-30} ");
            var result = TestDocument(docPath, forceOcr: false);

            if (result.Success)
            {
                Console.WriteLine($"OK   (text: {result.TextLength} chars)");
                results.Add(result);
            }
            else
            {
                Console.WriteLine("FAIL");
                Console.WriteLine($"      Error: {result.ErrorType}: {result.Error}");
                results.Add(result);
                allPassed = false;
            }
        }

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("OCR Tests (force_ocr=True)");
        Console.WriteLine(new string('-', 80));

        var ocrTestFiles = new (string Name, string Type)[]
        {
            ("tiny.pdf", "PDF with OCR"),
            ("ocr_image.jpg", "JPG Image with OCR")
        };

        foreach (var (docName, docType) in ocrTestFiles)
        {
            var docPath = Path.Combine(testDocsDir, docName);

            if (!File.Exists(docPath))
            {
                Console.WriteLine($"SKIP  {docType,-25} {docName,-30} - File not found");
                continue;
            }

            Console.Write($"TEST  {docType,-25} {docName,-30} ");
            var result = TestDocument(docPath, forceOcr: true);

            if (result.Success)
            {
                Console.WriteLine($"OK   (text: {result.TextLength} chars)");
                results.Add(result);
                if (result.TextLength == 0)
                {
                    Console.WriteLine("      WARNING: OCR extracted 0 characters - PDFium may not be bundled correctly");
                }
            }
            else
            {
                Console.WriteLine("FAIL");
                Console.WriteLine($"      Error: {result.ErrorType}: {result.Error}");
                results.Add(result);
                allPassed = false;
            }
        }

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Summary");
        Console.WriteLine(new string('-', 80));

        var passed = results.Count(r => r.Success);
        var failed = results.Count(r => !r.Success);

        Console.WriteLine($"Passed: {passed}/{results.Count}");
        Console.WriteLine($"Failed: {failed}/{results.Count}");

        if (failed > 0)
        {
            Console.WriteLine("\nFailed tests:");
            foreach (var r in results.Where(r => !r.Success))
            {
                Console.WriteLine($"  - {r.File}: {r.Error}");
            }
        }

        Console.WriteLine("\nDetailed Results:");
        var options = new JsonSerializerOptions { WriteIndented = true };
        Console.WriteLine(JsonSerializer.Serialize(results, options));

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Output Format Tests");
        Console.WriteLine(new string('-', 80));

        // Test output_format values
        var outputFormats = new[] { "plain", "markdown", "djot", "html" };
        foreach (var format in outputFormats)
        {
            Console.Write($"TEST  OutputFormat={format,-15} ");
            try
            {
                var config = new ExtractionConfig { OutputFormat = format };
                if (config.OutputFormat == format)
                {
                    Console.WriteLine("OK");
                }
                else
                {
                    Console.WriteLine($"FAIL - Expected {format}, got {config.OutputFormat}");
                    allPassed = false;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"FAIL - {ex.Message}");
                allPassed = false;
            }
        }

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Result Format Tests");
        Console.WriteLine(new string('-', 80));

        // Test result_format values
        var resultFormats = new[] { "unified", "element_based" };
        foreach (var format in resultFormats)
        {
            Console.Write($"TEST  ResultFormat={format,-15} ");
            try
            {
                var config = new ExtractionConfig { ResultFormat = format };
                if (config.ResultFormat == format)
                {
                    Console.WriteLine("OK");
                }
                else
                {
                    Console.WriteLine($"FAIL - Expected {format}, got {config.ResultFormat}");
                    allPassed = false;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"FAIL - {ex.Message}");
                allPassed = false;
            }
        }

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Format Combination Tests");
        Console.WriteLine(new string('-', 80));

        // Test format combinations
        foreach (var outputFmt in outputFormats)
        {
            foreach (var resultFmt in resultFormats)
            {
                Console.Write($"TEST  {outputFmt}+{resultFmt,-20} ");
                try
                {
                    var config = new ExtractionConfig
                    {
                        OutputFormat = outputFmt,
                        ResultFormat = resultFmt
                    };
                    if (config.OutputFormat == outputFmt && config.ResultFormat == resultFmt)
                    {
                        Console.WriteLine("OK");
                    }
                    else
                    {
                        Console.WriteLine($"FAIL - Config mismatch");
                        allPassed = false;
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"FAIL - {ex.Message}");
                    allPassed = false;
                }
            }
        }

        Console.WriteLine(new string('-', 80));
        Console.WriteLine("Format Extraction Tests");
        Console.WriteLine(new string('-', 80));

        // Test extraction with formats
        var docxPath = Path.Combine(testDocsDir, "lorem_ipsum.docx");
        if (File.Exists(docxPath))
        {
            foreach (var outputFmt in outputFormats)
            {
                Console.Write($"TEST  Extract with output_format={outputFmt,-10} ");
                try
                {
                    var config = new ExtractionConfig { OutputFormat = outputFmt };
                    var extractResult = KreuzbergClient.ExtractFileSync(docxPath, config);
                    if (extractResult != null && extractResult.Content != null)
                    {
                        Console.WriteLine($"OK   (text: {extractResult.Content.Length} chars)");
                    }
                    else
                    {
                        Console.WriteLine("FAIL - No content returned");
                        allPassed = false;
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"FAIL - {ex.Message}");
                    allPassed = false;
                }
            }

            foreach (var resultFmt in resultFormats)
            {
                Console.Write($"TEST  Extract with result_format={resultFmt,-15} ");
                try
                {
                    var config = new ExtractionConfig { ResultFormat = resultFmt };
                    var extractResult = KreuzbergClient.ExtractFileSync(docxPath, config);
                    if (extractResult != null)
                    {
                        Console.WriteLine($"OK   (text: {extractResult.Content?.Length ?? 0} chars)");
                    }
                    else
                    {
                        Console.WriteLine("FAIL - No result returned");
                        allPassed = false;
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"FAIL - {ex.Message}");
                    allPassed = false;
                }
            }
        }
        else
        {
            Console.WriteLine("SKIP  Format extraction tests - lorem_ipsum.docx not found");
        }

        return allPassed;
    }
}
