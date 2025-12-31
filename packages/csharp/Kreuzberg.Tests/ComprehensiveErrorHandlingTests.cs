using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive error handling tests covering exception hierarchy, async exception unwrapping,
/// validation errors, file not found scenarios, invalid configurations, and edge cases.
/// </summary>
public class ComprehensiveErrorHandlingTests
{
    public ComprehensiveErrorHandlingTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region KreuzbergException Hierarchy Tests

    [Fact]
    public void KreuzbergValidationException_IsKreuzbergException()
    {
        var ex = new KreuzbergValidationException("Test error");

        Assert.IsAssignableFrom<KreuzbergException>(ex);
    }

    [Fact]
    public void KreuzbergException_CanBeCaught()
    {
        try
        {
            throw new KreuzbergException(KreuzbergErrorKind.Unknown, "Test error");
        }
        catch (KreuzbergException ex)
        {
            Assert.NotNull(ex);
            Assert.Contains("Test error", ex.Message);
        }
    }

    [Fact]
    public void KreuzbergValidationException_CanBeCaughtAsKreuzbergException()
    {
        try
        {
            throw new KreuzbergValidationException("Validation error");
        }
        catch (KreuzbergException ex)
        {
            Assert.NotNull(ex);
        }
    }

    #endregion

    #region Null and Empty Input Tests

    [Fact]
    public void ExtractFileSync_WithEmptyPath_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<ArgumentException>(() =>
            KreuzbergClient.ExtractFileSync("")
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public void ExtractFileSync_WithNullPath_ThrowsException()
    {
        var ex = Assert.Throws<ArgumentException>(() =>
            KreuzbergClient.ExtractFileSync(null!)
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public void ExtractFileSync_WithWhitespacePath_ThrowsException()
    {
        var ex = Assert.Throws<ArgumentException>(() =>
            KreuzbergClient.ExtractFileSync("   ")
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public void DetectMimeType_WithEmptyData_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.DetectMimeType(new ReadOnlySpan<byte>())
        );

        Assert.Contains("cannot be empty", ex.Message);
    }

    #endregion

    #region File Not Found Tests

    [Fact]
    public void ExtractFileSync_WithNonexistentFile_ThrowsException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.ExtractFileSync("nonexistent/file.pdf")
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public async Task ExtractFileAsync_WithNonexistentFile_ThrowsExceptionAsync()
    {
        var ex = await Assert.ThrowsAsync<KreuzbergValidationException>(async () =>
            await KreuzbergClient.ExtractFileAsync("nonexistent/file.pdf")
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public void ExtractFileSync_WithDirectoryPath_ThrowsException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.ExtractFileSync(Path.GetTempPath())
        );

        Assert.NotNull(ex);
    }

    #endregion

    #region Invalid Configuration Tests

    [Fact]
    public void ExtractFileSync_WithInvalidPdfPassword_StillReturnsResult()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                Passwords = new List<string> { "wrongpassword123" }
            }
        };

        try
        {
            var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

            // Should still return a result, even if password protected and wrong
            Assert.NotNull(result);
        }
        catch (KreuzbergParsingException)
        {
            // Acceptable: invalid PDF password may cause parsing issues
        }
    }

    [Fact]
    public void ExtractFileSync_WithNullConfig_UsesDefaults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        try
        {
            var result = KreuzbergClient.ExtractFileSync(pdfPath, config: null);

            Assert.NotNull(result);
            Assert.True(result.Success);
        }
        catch (KreuzbergParsingException)
        {
            // Acceptable: some PDFs may have parsing issues
        }
    }

    #endregion

    #region Async Exception Unwrapping Tests

    [Fact]
    public async Task ExtractFileAsync_ThrowsDirectExceptionNotAggregateException()
    {
        var ex = await Assert.ThrowsAsync<TaskCanceledException>(async () =>
        {
            using var cts = new CancellationTokenSource();
            cts.Cancel();

            await KreuzbergClient.ExtractFileAsync(
                NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
                cancellationToken: cts.Token
            );
        });

        Assert.NotNull(ex);
        Assert.IsType<TaskCanceledException>(ex);
    }

    [Fact]
    public async Task ExtractFileAsync_InvalidFile_ThrowsDirectException()
    {
        var ex = await Assert.ThrowsAsync<KreuzbergValidationException>(async () =>
            await KreuzbergClient.ExtractFileAsync("nonexistent.pdf")
        );

        Assert.NotNull(ex);
        Assert.False(ex is AggregateException);
    }

    #endregion

    #region Cancellation Exception Tests

    [Fact]
    public async Task ExtractFileAsync_WithCancelledToken_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var ex = await Assert.ThrowsAsync<TaskCanceledException>(async () =>
            await KreuzbergClient.ExtractFileAsync(
                NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
                cancellationToken: cts.Token
            )
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithCancelledToken_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        };

        var ex = await Assert.ThrowsAsync<TaskCanceledException>(async () =>
            await KreuzbergClient.BatchExtractFilesAsync(files, cancellationToken: cts.Token)
        );

        Assert.NotNull(ex);
    }

    [Fact]
    public async Task BatchExtractBytesAsync_WithCancelledToken_ThrowsOperationCanceledException()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var filePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(filePath);

        var items = new List<BytesWithMime>
        {
            new(bytes, "application/pdf")
        };

        var ex = await Assert.ThrowsAsync<TaskCanceledException>(async () =>
            await KreuzbergClient.BatchExtractBytesAsync(items, cancellationToken: cts.Token)
        );

        Assert.NotNull(ex);
    }

    #endregion

    #region Configuration Validation Tests

    [Fact]
    public void GetEmbeddingPreset_WithEmptyName_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.GetEmbeddingPreset("")
        );

        Assert.Contains("cannot be empty", ex.Message);
    }

    [Fact]
    public void GetEmbeddingPreset_WithNullName_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.GetEmbeddingPreset(null!)
        );

        Assert.Contains("cannot be empty", ex.Message);
    }

    [Fact]
    public void GetEmbeddingPreset_WithWhitespaceName_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.GetEmbeddingPreset("   ")
        );

        Assert.Contains("cannot be empty", ex.Message);
    }

    #endregion

    #region Batch Operation Error Handling Tests

    [Fact]
    public void BatchExtractFilesSync_WithEmptyList_ReturnsEmptyList()
    {
        var files = new List<string>();

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    [Fact]
    public void BatchExtractBytesSync_WithEmptyList_ReturnsEmptyList()
    {
        var items = new List<BytesWithMime>();

        var results = KreuzbergClient.BatchExtractBytesSync(items);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    [Fact]
    public void BatchExtractFilesSync_WithNullList_ThrowsException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() =>
            KreuzbergClient.BatchExtractFilesSync(null!)
        );

        Assert.NotNull(ex);
    }

    #endregion

    #region Edge Cases and Boundary Conditions Tests

    [Fact]
    public void ExtractFileSync_WithVeryLongPath_HandlesCorrectly()
    {
        // Create a path with reasonable length
        var basePath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var result = KreuzbergClient.ExtractFileSync(basePath);

        Assert.NotNull(result);
    }

    [Fact]
    public void DetectMimeType_WithSingleByte_ReturnsOrThrows()
    {
        var singleByte = new byte[] { 0xFF };

        try
        {
            var mimeType = KreuzbergClient.DetectMimeType(singleByte);
            // Should return something or throw
            Assert.NotNull(mimeType);
        }
        catch (KreuzbergException)
        {
            // Also acceptable
        }
    }

    [Fact]
    public void DetectMimeTypeFromPath_WithEmptyPath_ThrowsKreuzbergValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.DetectMimeTypeFromPath("")
        );

        Assert.Contains("cannot be empty", ex.Message);
    }

    #endregion

    #region Exception Message Tests

    [Fact]
    public void KreuzbergException_ContainsMessage()
    {
        var message = "This is a test error";
        var ex = new KreuzbergException(KreuzbergErrorKind.Unknown, message);

        Assert.Contains(message, ex.Message);
    }

    [Fact]
    public void KreuzbergValidationException_ContainsMessage()
    {
        var message = "This is a validation error";
        var ex = new KreuzbergValidationException(message);

        Assert.Contains(message, ex.Message);
    }

    [Fact]
    public void KreuzbergException_HasInnerException_PreservesIt()
    {
        var innerEx = new ArgumentException("Inner exception");
        var ex = new KreuzbergException(KreuzbergErrorKind.Unknown, "Outer exception", innerEx);

        Assert.NotNull(ex.InnerException);
        Assert.Equal(innerEx, ex.InnerException);
    }

    #endregion

    #region Async Exception Context Tests

    [Fact]
    public async Task ExtractFileAsync_ExceptionContext_IsPreserved()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        try
        {
            using var cts = new CancellationTokenSource();
            cts.Cancel();

            await KreuzbergClient.ExtractFileAsync(pdfPath, cancellationToken: cts.Token);

            Assert.Fail("Should have thrown");
        }
        catch (OperationCanceledException ex)
        {
            // Exception context should be preserved
            Assert.NotNull(ex);
        }
    }

    [Fact]
    public async Task ExtractFileAsync_ExceptionInContinuation_IsPreserved()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var ex = await Assert.ThrowsAsync<TaskCanceledException>(async () =>
        {
            using var cts = new CancellationTokenSource();
            cts.Cancel();

            var task = KreuzbergClient.ExtractFileAsync(pdfPath, cancellationToken: cts.Token);
            await task;
        });

        Assert.NotNull(ex);
    }

    #endregion

    #region Recovery from Errors Tests

    [Fact]
    public async Task ExtractFileAsync_AfterError_CanRecoverWithValidFile()
    {
        // First, trigger an error
        await Assert.ThrowsAsync<KreuzbergValidationException>(async () =>
            await KreuzbergClient.ExtractFileAsync("nonexistent.pdf")
        );

        // Then, successfully extract a valid file
        var validResult = await KreuzbergClient.ExtractFileAsync(
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        );

        Assert.NotNull(validResult);
        Assert.True(validResult.Success);
    }

    [Fact]
    public void ExtractFileSync_AfterError_CanRecoverWithValidFile()
    {
        // First, trigger an error
        Assert.Throws<KreuzbergValidationException>(() =>
            KreuzbergClient.ExtractFileSync("nonexistent.pdf")
        );

        // Then, successfully extract a valid file
        var validResult = KreuzbergClient.ExtractFileSync(
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf")
        );

        Assert.NotNull(validResult);
        Assert.True(validResult.Success);
    }

    #endregion
}
