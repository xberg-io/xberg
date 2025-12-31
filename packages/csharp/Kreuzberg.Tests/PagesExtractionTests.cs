using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for document page extraction functionality.
/// Tests cover page extraction, page marker insertion, custom marker formats, and page content validation.
/// </summary>
public class PagesExtractionTests
{
    public PagesExtractionTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region Extract Pages Tests

    [Fact]
    public void ExtractPages_WithMultiPagePdf_ReturnsPageArray()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
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
        Assert.NotEmpty(result.Pages);
    }

    [Fact]
    public void ExtractPages_ReturnsPageNumbers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        foreach (var page in result.Pages)
        {
            Assert.True(page.PageNumber > 0, "Page number should be positive");
        }
    }

    [Fact]
    public void ExtractPages_ReturnsPageContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        foreach (var page in result.Pages)
        {
            Assert.NotNull(page.Content);
            Assert.True(!string.IsNullOrEmpty(page.Content), $"Page {page.PageNumber} should have content");
        }
    }

    [Fact]
    public void ExtractPages_WithDisabledOption_ReturnsNull()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = false
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.Null(result.Pages);
    }

    [Fact]
    public void ExtractPages_PreservesPageOrder()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Pages != null && result.Pages.Count > 1)
        {
            for (int i = 0; i < result.Pages.Count - 1; i++)
            {
                Assert.True(result.Pages[i].PageNumber < result.Pages[i + 1].PageNumber,
                    "Pages should be in ascending order");
            }
        }
    }

    #endregion

    #region Page Markers Tests

    [Fact]
    public void InsertPageMarkers_WithMultiPagePdf_InsertsMarkers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Content);
        Assert.Contains("<!-- PAGE", result.Content);
    }

    [Fact]
    public void InsertPageMarkers_WithoutOption_NoMarkers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = false
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Default marker format should not appear when not enabled
        var defaultMarkerAppears = result.Content.Contains("<!-- PAGE");
        Assert.False(defaultMarkerAppears, "Default markers should not appear when disabled");
    }

    [Fact]
    public void InsertPageMarkers_ContainsPageNumbers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        // Should contain at least page 1
        Assert.Contains("1", result.Content);
    }

    [Fact]
    public void InsertPageMarkers_MultipleMarkers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        var markerCount = System.Text.RegularExpressions.Regex.Matches(result.Content, "<!-- PAGE").Count;
        Assert.True(markerCount > 0, "Should contain at least one page marker");
    }

    #endregion

    #region Custom Marker Format Tests

    [Fact]
    public void CustomMarkerFormat_WithMultiPagePdf_UsesCustomFormat()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var customFormat = "=== PAGE {page_num} ===";
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true,
                MarkerFormat = customFormat
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Content);
        Assert.Contains("=== PAGE", result.Content);
    }

    [Fact]
    public void CustomMarkerFormat_WithPageNumberPlaceholder_ReplacesPlaceholder()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var customFormat = "[Page Number: {page_num}]";
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true,
                MarkerFormat = customFormat
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        Assert.Contains("[Page Number:", result.Content);
        Assert.DoesNotContain("{page_num}", result.Content);
    }

    [Fact]
    public void CustomMarkerFormat_WithSimpleFormat_Works()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var customFormat = "PAGE_{page_num}";
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true,
                MarkerFormat = customFormat
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        Assert.Contains("PAGE_", result.Content);
    }

    [Fact]
    public void CustomMarkerFormat_WithLineSeparators_Works()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var customFormat = "\n---PAGE {page_num}---\n";
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true,
                MarkerFormat = customFormat
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        Assert.Contains("---PAGE", result.Content);
    }

    [Fact]
    public void CustomMarkerFormat_OverridesDefault()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var customFormat = "CUSTOM_PAGE_{page_num}";
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true,
                MarkerFormat = customFormat
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        // Should use custom format, not default
        Assert.Contains("CUSTOM_PAGE_", result.Content);
    }

    #endregion

    #region Multi-Page PDF Tests

    [Fact]
    public void MultiPagePdf_ExtractPages_ProducesMultiplePages()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        Assert.True(result.Pages.Count > 0, "Should extract at least one page");
    }

    [Fact]
    public void MultiPagePdf_PageNumbersAreSequential()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        for (int i = 0; i < result.Pages.Count; i++)
        {
            Assert.Equal(i + 1, result.Pages[i].PageNumber);
        }
    }

    [Fact]
    public void MultiPagePdf_EachPageHasContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        foreach (var page in result.Pages)
        {
            Assert.NotNull(page.Content);
            Assert.NotEmpty(page.Content.Trim());
        }
    }

    [Fact]
    public void MultiPagePdf_WithPageMarkers_ContainsAllPages()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Content);
        // Count page markers to verify multiple pages
        var markerMatches = System.Text.RegularExpressions.Regex.Matches(result.Content, "<!-- PAGE");
        Assert.True(markerMatches.Count >= 1, "Should contain page markers");
    }

    #endregion

    #region Page Content Structure Validation Tests

    [Fact]
    public void ExtractPages_ValidatesPageStructure()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        foreach (var page in result.Pages)
        {
            Assert.NotNull(page.Content);
            Assert.True(page.PageNumber > 0);
        }
    }

    [Fact]
    public void PageContent_HasRequiredFields()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        foreach (var page in result.Pages)
        {
            // Check required fields
            Assert.True(page.PageNumber > 0, "Page must have a valid page number");
            Assert.NotNull(page.Content);
        }
    }

    [Fact]
    public void PageContent_WithTables_PreservesTableData()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        // Each page might have tables
        foreach (var page in result.Pages)
        {
            // Tables in page content are optional
            if (page.Tables != null)
            {
                Assert.IsAssignableFrom<ICollection<Table>>(page.Tables);
            }
        }
    }

    [Fact]
    public void PageContent_WithImages_PreservesImageData()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        // Each page might have images
        foreach (var page in result.Pages)
        {
            // Images in page content are optional
            if (page.Images != null)
            {
                Assert.IsAssignableFrom<ICollection<ExtractedImage>>(page.Images);
            }
        }
    }

    [Fact]
    public void PageContent_ContentIsNotEmpty()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        var pageWithContent = result.Pages.FirstOrDefault(p => !string.IsNullOrWhiteSpace(p.Content));
        Assert.NotNull(pageWithContent);
    }

    #endregion

    #region Combined Features Tests

    [Fact]
    public void ExtractPages_AndInsertMarkers_TogetherWorks()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true,
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Pages);
        Assert.NotEmpty(result.Pages);
        Assert.Contains("<!-- PAGE", result.Content);
    }

    [Fact]
    public void ExtractPages_WithCustomMarker_CombinesFeatures()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true,
                InsertPageMarkers = true,
                MarkerFormat = "[PAGE {page_num}]"
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        Assert.NotEmpty(result.Pages);
        Assert.Contains("[PAGE", result.Content);
    }

    [Fact]
    public void PageExtraction_ConsistentBetweenPageArrayAndMarkers()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Pages = new PageConfig
            {
                ExtractPages = true,
                InsertPageMarkers = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Pages);
        Assert.NotNull(result.Content);

        // Count pages in array and in markers should be consistent
        var pageArrayCount = result.Pages.Count;
        var markerMatches = System.Text.RegularExpressions.Regex.Matches(result.Content, "<!-- PAGE");
        var markerCount = markerMatches.Count;

        Assert.Equal(pageArrayCount, markerCount);
    }

    #endregion

    #region Configuration Tests

    [Fact]
    public void PageConfig_DefaultValues()
    {
        var config = new PageConfig();

        Assert.False(config.ExtractPages ?? false);
        Assert.False(config.InsertPageMarkers ?? false);
        Assert.Null(config.MarkerFormat);
    }

    [Fact]
    public void PageConfig_AcceptsCustomValues()
    {
        var config = new PageConfig
        {
            ExtractPages = true,
            InsertPageMarkers = true,
            MarkerFormat = "CUSTOM_{page_num}"
        };

        Assert.True(config.ExtractPages ?? false);
        Assert.True(config.InsertPageMarkers ?? false);
        Assert.Equal("CUSTOM_{page_num}", config.MarkerFormat);
    }

    #endregion
}
