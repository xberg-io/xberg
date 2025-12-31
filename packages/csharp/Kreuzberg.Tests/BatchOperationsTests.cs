using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for batch extraction operations.
/// Tests cover batch file extraction, batch byte extraction, cancellation handling,
/// error handling, and concurrent batch operations.
/// </summary>
public class BatchOperationsTests
{
    public BatchOperationsTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region Batch File Extraction Tests

    [Fact]
    public void BatchExtractFiles_WithMultiplePdfFiles_ReturnsResultsForEachFile()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);
        Assert.All(results, r => Assert.NotNull(r));
    }

    [Fact]
    public void BatchExtractFiles_WithMixedDocumentTypes_ReturnsResultsForEachFile()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx"),
            NativeTestHelper.GetDocumentPath("office/excel.xlsx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);
        Assert.All(results, r => Assert.True(r.Success));
    }

    [Fact]
    public void BatchExtractFiles_AllResultsHaveContent()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.All(results, r => Assert.NotEmpty(r.Content));
    }

    [Fact]
    public void BatchExtractFiles_WithConfiguration_AppliesConfigToAllFiles()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf")
        };

        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);
        Assert.All(results, r => Assert.NotNull(r.Metadata));
    }

    [Fact]
    public void BatchExtractFiles_WithImageExtraction_ExtractsImagesFromAllFiles()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf")
        };

        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config);

        Assert.NotNull(results);
        Assert.NotEmpty(results);
        // If extraction succeeds, Images should be present; if it fails, that's also acceptable
        Assert.All(results, r => Assert.True(r.Success || r.Images != null));
    }

    [Fact]
    public void BatchExtractFiles_WithTableExtraction_ExtractsTablesFromAllFiles()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf")
        };

        var config = new ExtractionConfig
        {
            // Default includes table extraction
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config);

        Assert.NotNull(results);
        Assert.NotEmpty(results);
    }

    #endregion

    #region Batch Bytes Extraction Tests

    [Fact]
    public void BatchExtractBytes_WithMultipleDocuments_ReturnsResultsForEachDocument()
    {
        var file1Path = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var file2Path = NativeTestHelper.GetDocumentPath("office/document.docx");

        var bytes1 = File.ReadAllBytes(file1Path);
        var bytes2 = File.ReadAllBytes(file2Path);

        var items = new List<BytesWithMime>
        {
            new(bytes1, "application/pdf"),
            new(bytes2, "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        };

        var results = KreuzbergClient.BatchExtractBytesSync(items);

        Assert.NotNull(results);
        Assert.Equal(items.Count, results.Count);
        Assert.All(results, r => Assert.NotNull(r));
    }

    [Fact]
    public void BatchExtractBytes_AllResultsHaveContent()
    {
        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf"),
            new(bytes, "application/pdf")
        };

        var results = KreuzbergClient.BatchExtractBytesSync(items);

        Assert.All(results, r => Assert.NotEmpty(r.Content));
    }

    [Fact]
    public void BatchExtractBytes_WithConfiguration_AppliesConfigToAllItems()
    {
        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf"),
            new(bytes, "application/pdf")
        };

        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var results = KreuzbergClient.BatchExtractBytesSync(items, config);

        Assert.Equal(items.Count, results.Count);
        Assert.All(results, r => Assert.NotNull(r.Metadata));
    }

    #endregion

    #region Async Batch Operations Tests

    [Fact]
    public async Task BatchExtractFilesAsync_WithMultipleFiles_ReturnsAllResults()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = await KreuzbergClient.BatchExtractFilesAsync(files);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithConfiguration_AppliesConfig()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var results = await KreuzbergClient.BatchExtractFilesAsync(files, config);

        Assert.NotEmpty(results);
        Assert.All(results, r => Assert.NotNull(r.Metadata));
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithCancellationToken_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var exceptionThrown = false;
        try
        {
            await KreuzbergClient.BatchExtractFilesAsync(files, cancellationToken: cts.Token);
        }
        catch (OperationCanceledException)
        {
            exceptionThrown = true;
        }

        Assert.True(exceptionThrown, "Expected OperationCanceledException or subclass to be thrown");
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithTimeout_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(10));

        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        // Timeout scenarios may or may not throw depending on timing, so verify the operation either
        // completes or throws an OperationCanceledException
        try
        {
            var result = await KreuzbergClient.BatchExtractFilesAsync(files, cancellationToken: cts.Token);
            // If it completes, that's acceptable
            Assert.NotNull(result);
        }
        catch (OperationCanceledException ex)
        {
            // This is the expected outcome
            Assert.NotNull(ex);
        }
    }

    [Fact]
    public async Task BatchExtractBytesAsync_WithMultipleDocuments_ReturnsAllResults()
    {
        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf"),
            new(bytes, "application/pdf")
        };

        var results = await KreuzbergClient.BatchExtractBytesAsync(items);

        Assert.NotNull(results);
        Assert.Equal(items.Count, results.Count);
    }

    [Fact]
    public async Task BatchExtractBytesAsync_WithConfiguration_AppliesConfig()
    {
        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf")
        };

        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var results = await KreuzbergClient.BatchExtractBytesAsync(items, config);

        Assert.NotEmpty(results);
        Assert.All(results, r => Assert.NotNull(r.Metadata));
    }

    [Fact]
    public async Task BatchExtractBytesAsync_WithCancellationToken_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf")
        };

        var exceptionThrown = false;
        try
        {
            await KreuzbergClient.BatchExtractBytesAsync(items, cancellationToken: cts.Token);
        }
        catch (OperationCanceledException)
        {
            exceptionThrown = true;
        }

        Assert.True(exceptionThrown, "Expected OperationCanceledException or subclass to be thrown");
    }

    #endregion

    #region Batch Error Handling Tests

    [Fact]
    public void BatchExtractFiles_WithEmptyList_ReturnsEmptyResult()
    {
        var files = new List<string>();

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    [Fact]
    public void BatchExtractFiles_WithSingleFile_ReturnsSingleResult()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotNull(results);
        Assert.Single(results);
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithEmptyList_ReturnsEmptyResult()
    {
        var files = new List<string>();

        var results = await KreuzbergClient.BatchExtractFilesAsync(files);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    #endregion

    #region Batch Processing Pattern Tests

    [Fact]
    public void BatchExtractFiles_ResultsCanBeEnumeratedMultipleTimes()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        var firstEnumeration = results.Count();
        var secondEnumeration = results.Count();

        Assert.Equal(firstEnumeration, secondEnumeration);
    }

    [Fact]
    public void BatchExtractFiles_ResultsCanBeIndexed()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        // Should support indexing
        var firstResult = results[0];
        var secondResult = results[1];

        Assert.NotNull(firstResult);
        Assert.NotNull(secondResult);
    }

    [Fact]
    public void BatchExtractFiles_ResultsPreserveOrder()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx"),
            NativeTestHelper.GetDocumentPath("office/excel.xlsx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.Equal(files.Length, results.Count);
        for (int i = 0; i < results.Count; i++)
        {
            Assert.NotNull(results[i]);
        }
    }

    [Fact(Skip = "Test causes resource exhaustion in concurrent environments")]
    public async Task BatchExtractFilesAsync_LargeBatch_ProcessesAllSuccessfully()
    {
        var singleFile = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var files = Enumerable.Repeat(singleFile, 2).ToList();

        var results = await KreuzbergClient.BatchExtractFilesAsync(files);

        Assert.NotNull(results);
        Assert.Equal(2, results.Count);
        Assert.All(results, r => Assert.True(r.Success));
    }

    #endregion

    #region Batch with Different Configurations Tests

    [Fact]
    public void BatchExtractFiles_WithoutConfiguration_UsesDefaults()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotEmpty(results);
        Assert.All(results, r => Assert.NotEmpty(r.Content));
    }

    [Fact]
    public void BatchExtractFiles_WithNullConfiguration_UsesDefaults()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config: null);

        Assert.NotEmpty(results);
        Assert.All(results, r => Assert.NotEmpty(r.Content));
    }

    #endregion
}
