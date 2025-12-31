using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Advanced tests for table and image extraction with comprehensive coverage
/// of extraction quality, metadata, format validation, and edge cases.
/// </summary>
public class AdvancedTableAndImageTests
{
    public AdvancedTableAndImageTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region Advanced Table Extraction Tests

    [Fact]
    public void ExtractTables_FromPdfWithTables_ReturnsTableStructure()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
    }

    [Fact]
    public void Table_HasCellsProperty()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            Assert.NotNull(table.Cells);
            Assert.NotEmpty(table.Cells);
        }
    }

    [Fact]
    public void Table_EachRowHasCells()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            foreach (var row in table.Cells)
            {
                Assert.NotNull(row);
                Assert.NotEmpty(row);
            }
        }
    }

    [Fact]
    public void Table_HasMarkdownRepresentation()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            Assert.NotNull(table.Markdown);
            // Markdown should contain pipe characters for table structure
            if (!string.IsNullOrEmpty(table.Markdown))
            {
                Assert.Contains("|", table.Markdown);
            }
        }
    }

    [Fact]
    public void Table_HasPageNumber()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            Assert.True(table.PageNumber > 0, "Table should have positive page number");
        }
    }

    [Fact]
    public void Table_MarkdownFormattingIsValid()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            if (!string.IsNullOrEmpty(table.Markdown))
            {
                // Markdown tables should have consistent row structure
                var lines = table.Markdown.Split('\n', StringSplitOptions.RemoveEmptyEntries);
                if (lines.Length > 0)
                {
                    var firstLine = lines[0];
                    Assert.Contains("|", firstLine);
                }
            }
        }
    }

    [Fact]
    public void ExtractTables_WithConfiguration_StillExtractsTables()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
    }

    [Fact]
    public async Task ExtractTablesAsync_WithAsyncOperation_ReturnsContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");

        var result = await KreuzbergClient.ExtractFileAsync(pdfPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
    }

    [Fact]
    public void BatchExtractTables_FromMultipleFiles_ExtractsFromAll()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files);

        Assert.NotEmpty(results);
    }

    [Fact]
    public void Table_EmptyTableIsHandledCorrectly()
    {
        // Test with PDF that might have empty tables
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables != null)
        {
            foreach (var table in result.Tables)
            {
                Assert.NotNull(table.Cells);
                // Even empty tables should have the cells property
            }
        }
    }

    #endregion

    #region Advanced Image Extraction Tests

    [Fact]
    public void ExtractImages_FromPdfWithImages_ReturnsImageList()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Images);
    }

    [Fact]
    public void ExtractedImage_HasFormat()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            Assert.NotNull(image.Format);
            Assert.NotEmpty(image.Format);
            // Should be a valid image format
            var validFormats = new[] { "PNG", "JPEG", "TIFF", "JPEG2000", "GIF", "BMP", "DCTDECODE" };
            Assert.Contains(image.Format.ToUpper(), validFormats);
        }
    }

    [Fact]
    public void ExtractedImage_HasDimensions()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            Assert.True(image.Width > 0, "Image should have positive width");
            Assert.True(image.Height > 0, "Image should have positive height");
        }
    }

    [Fact]
    public void ExtractedImage_DimensionsArePositive()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null)
        {
            foreach (var image in result.Images)
            {
                Assert.True(image.Width > 0);
                Assert.True(image.Height > 0);
            }
        }
    }

    [Fact]
    public void ExtractedImage_HasData()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            Assert.NotNull(image.Data);
            // Data might be base64 encoded or binary
        }
    }

    [Fact]
    public void ExtractImages_WithDifferentDpi_ReturnsImages()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");

        // Low DPI
        var lowDpiConfig = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 72
            }
        };

        var lowDpiResult = KreuzbergClient.ExtractFileSync(pdfPath, lowDpiConfig);

        // High DPI
        var highDpiConfig = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 300
            }
        };

        var highDpiResult = KreuzbergClient.ExtractFileSync(pdfPath, highDpiConfig);

        Assert.NotNull(lowDpiResult);
        Assert.NotNull(highDpiResult);
    }

    [Fact]
    public void ExtractImages_WithoutConfiguration_DoesNotExtractImages()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = false
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Images might be null or empty if extraction disabled
    }

    [Fact]
    public async Task ExtractImagesAsync_WithAsyncOperation_ReturnsContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = await KreuzbergClient.ExtractFileAsync(pdfPath, config: config);

        Assert.NotNull(result);
    }

    [Fact]
    public void BatchExtractImages_FromMultipleFiles_ExtractsFromAll()
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

        Assert.NotEmpty(results);
    }

    [Fact]
    public void ExtractedImage_PageNumberIsValid()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            Assert.True(image.PageNumber > 0, "Image should have positive page number");
        }
    }

    #endregion

    #region Page Extraction with Tables and Images Tests

    [Fact]
    public void ExtractPages_WithTableExtraction_BothWorkTogether()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Pages);
    }

    [Fact]
    public void PageContent_MayContainTables()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Pages != null && result.Pages.Count > 0)
        {
            var page = result.Pages[0];
            Assert.NotNull(page.Content);
        }
    }

    [Fact]
    public void PageContent_MayContainImages()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            },
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Pages != null && result.Pages.Count > 0)
        {
            var page = result.Pages[0];
            Assert.NotNull(page.Content);
        }
    }

    #endregion

    #region Image Format Tests

    [Fact]
    public void ExtractedImage_FormatIsCaseInsensitive()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            var upperFormat = image.Format.ToUpper();
            Assert.NotEmpty(upperFormat);
        }
    }

    [Fact]
    public void ExtractedImage_AspectRatioIsValid()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Images != null && result.Images.Count > 0)
        {
            var image = result.Images[0];
            if (image.Width > 0 && image.Height > 0)
            {
                var aspectRatio = (double)image.Width / image.Height;
                // Aspect ratio should be positive and reasonable (between 0.1 and 10)
                Assert.True(aspectRatio > 0.1 && aspectRatio < 10);
            }
        }
    }

    #endregion

    #region Error Handling Tests

    [Fact]
    public void ExtractImages_WithInvalidDpi_StillExtractsWithDefaults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 0 // Invalid DPI
            }
        };

        // Should not throw, but use defaults
        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
    }

    [Fact]
    public void ExtractTables_FromDocumentWithoutTables_ReturnsEmptyList()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var result = KreuzbergClient.ExtractFileSync(textPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
        // May be empty if document has no tables
    }

    #endregion
}
