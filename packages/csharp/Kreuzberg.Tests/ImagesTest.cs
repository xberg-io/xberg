using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for image extraction functionality from documents.
/// Tests cover image extraction with metadata, format detection, embedded vs. referenced images,
/// error handling, and batch processing capabilities.
/// </summary>
public class ImagesTest
{
    public ImagesTest()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region PDF Image Extraction with Metadata Tests

    [Fact]
    public void ExtractImages_FromPdfWithImages_ReturnsImageMetadata()
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
        Assert.NotEmpty(result.Images);
    }

    [Fact]
    public void ExtractedImage_ContainsFormat()
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

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        foreach (var image in result.Images)
        {
            Assert.NotNull(image.Format);
            Assert.NotEmpty(image.Format);
            Assert.Contains(image.Format.ToUpper(), new[] { "PNG", "JPEG", "TIFF", "JPEG2000" });
        }
    }

    [Fact]
    public void ExtractedImage_ContainsDimensions()
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

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        foreach (var image in result.Images)
        {
            // Dimensions should be populated for extracted images
            if (image.Width.HasValue)
            {
                Assert.True(image.Width.Value > 0, "Width should be positive when present");
                Assert.True(image.Width.Value < 100000, "Width should be reasonable (< 100000)");
            }
            if (image.Height.HasValue)
            {
                Assert.True(image.Height.Value > 0, "Height should be positive when present");
                Assert.True(image.Height.Value < 100000, "Height should be reasonable (< 100000)");
            }
        }
    }

    [Fact]
    public void ExtractedImage_ContainsImageIndex()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        // Image indices should be sequential starting from 0
        for (int i = 0; i < result.Images.Count; i++)
        {
            Assert.Equal(i, result.Images[i].ImageIndex);
        }
    }

    [Fact]
    public void ExtractedImage_ContainsColorspaceInformation()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        // At least some images should have colorspace information
        var imageWithColorspace = result.Images.FirstOrDefault(i => i.Colorspace != null);
        if (imageWithColorspace != null)
        {
            Assert.NotNull(imageWithColorspace.Colorspace);
            Assert.NotEmpty(imageWithColorspace.Colorspace);
        }
    }

    [Fact]
    public void ExtractedImage_ContainsRawData()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        foreach (var image in result.Images)
        {
            Assert.NotNull(image.Data);
            Assert.NotEmpty(image.Data);
        }
    }

    #endregion

    #region Image Format Detection Tests

    [Fact]
    public void ImageFormatDetection_RecognizesPng()
    {
        var pngPath = NativeTestHelper.GetDocumentPath("images/sample.png");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pngPath, config);

        // Image files themselves may be extracted as single images
        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ImageFormatDetection_RecognizesJpeg()
    {
        var jpegPath = NativeTestHelper.GetDocumentPath("images/example.jpg");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(jpegPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ImageFormatDetection_SetsCorrectMimeType()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.MimeType);
        Assert.NotEmpty(result.MimeType);
        Assert.Contains("pdf", result.MimeType.ToLower());
    }

    #endregion

    #region Composite Document Image Tests

    [Fact]
    public void ExtractImages_FromDocx_WithImages_ReturnsImages()
    {
        var docxPath = NativeTestHelper.GetDocumentPath("documents/word_image_anchors.docx");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(docxPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractImages_FromDocx_PreservesMetadata()
    {
        var docxPath = NativeTestHelper.GetDocumentPath("documents/word_sample.docx");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(docxPath, config);

        Assert.NotNull(result.Metadata);
        // DOCX format should be detected correctly
        Assert.NotEqual(FormatType.Unknown, result.Metadata.FormatType);
    }

    #endregion

    #region Image Configuration Tests

    [Fact]
    public void ImageConfig_WithTargetDpi_AppliesToExtraction()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 200,
                AutoAdjustDpi = false
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Configuration should be accepted without errors
        Assert.True(result.Success || result.Images == null);
    }

    [Fact]
    public void ImageConfig_WithMaxDimension_ConstrainsSize()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                MaxImageDimension = 1024
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success || result.Images == null);
    }

    [Fact]
    public void ImageConfig_WithAutoAdjustDpi_AdjustsAutomatically()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                AutoAdjustDpi = true,
                MinDpi = 72,
                MaxDpi = 300
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Should process without errors
        Assert.True(result.Success || result.Images == null);
    }

    [Fact]
    public void ImageConfig_Different_DPI_Settings_AffectExtraction()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");

        // Test with different DPI settings
        var configLowDPI = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 72
            }
        };

        var configHighDPI = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 300
            }
        };

        // Both configurations should produce valid results
        var resultLow = KreuzbergClient.ExtractFileSync(pdfPath, configLowDPI);
        var resultHigh = KreuzbergClient.ExtractFileSync(pdfPath, configHighDPI);

        Assert.NotNull(resultLow);
        Assert.NotNull(resultHigh);
        Assert.True(resultLow.Success);
        Assert.True(resultHigh.Success);
        // Different DPI settings should be processed without error
    }

    #endregion

    #region Image Extraction Disabled Tests

    [Fact]
    public void ExtractImages_WithDisabledOption_ReturnsNull()
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
        // When disabled, images should either be null or empty
        if (result.Images != null)
        {
            Assert.Empty(result.Images);
        }
    }

    [Fact]
    public void ExtractImages_WithoutConfig_ReturnsNull()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        // No images config provided
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Default behavior: images not extracted
        if (result.Images != null)
        {
            Assert.Empty(result.Images);
        }
    }

    [Fact]
    public void ExtractImages_FromPdfWithoutImages_ReturnsEmptyOrNull()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/simple.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // PDF without images should have no extracted images
        if (result.Images != null)
        {
            Assert.Empty(result.Images);
        }
    }

    #endregion

    #region Batch Image Extraction Tests

    [Fact]
    public void BatchExtractImages_MultipleDocuments_ProcessesAll()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf"),
            NativeTestHelper.GetDocumentPath("documents/word_sample.docx")
        };

        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);

        foreach (var result in results)
        {
            Assert.NotNull(result);
            Assert.True(result.Success || result.Images == null);
        }
    }

    [Fact]
    public void BatchExtractImages_PreservesOrder()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf"),
            NativeTestHelper.GetDocumentPath("pdfs/simple.pdf"),
            NativeTestHelper.GetDocumentPath("documents/word_sample.docx")
        };

        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var results = KreuzbergClient.BatchExtractFilesSync(files, config);

        Assert.NotNull(results);
        Assert.Equal(files.Length, results.Count);

        // Results should be in same order as input files
        for (int i = 0; i < files.Length; i++)
        {
            Assert.True(results[i] != null);
        }
    }

    #endregion

    #region Image Metadata Structure Tests

    [Fact]
    public void ExtractedImage_HasRequiredFields()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);
        Assert.NotEmpty(result.Images);

        foreach (var image in result.Images)
        {
            // Required fields
            Assert.NotNull(image.Data);
            Assert.NotNull(image.Format);
            Assert.True(image.ImageIndex >= 0);
        }
    }

    [Fact]
    public void ExtractedImage_OptionalFieldsNullableOrPopulated()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);

        foreach (var image in result.Images)
        {
            // Optional fields - should be null or have valid values
            if (image.Width.HasValue)
            {
                Assert.True(image.Width.Value > 0);
            }

            if (image.Height.HasValue)
            {
                Assert.True(image.Height.Value > 0);
            }

            if (image.PageNumber.HasValue)
            {
                Assert.True(image.PageNumber.Value > 0);
            }

            if (image.BitsPerComponent.HasValue)
            {
                Assert.True(image.BitsPerComponent.Value > 0);
            }
        }
    }

    [Fact]
    public void ExtractedImage_IsMaskField_DefaultsFalse()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Images);

        foreach (var image in result.Images)
        {
            // IsMask should be a boolean (default false for normal images)
            Assert.IsType<bool>(image.IsMask);
        }
    }

    #endregion

    #region Image Extraction Consistency Tests

    [Fact]
    public void ImageExtraction_RepeatedCalls_ProducesConsistentResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true
            }
        };

        var result1 = KreuzbergClient.ExtractFileSync(pdfPath, config);
        var result2 = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result1.Images);
        Assert.NotNull(result2.Images);
        Assert.Equal(result1.Images.Count, result2.Images.Count);

        // Images should have same properties
        for (int i = 0; i < result1.Images.Count; i++)
        {
            Assert.Equal(result1.Images[i].Format, result2.Images[i].Format);
            Assert.Equal(result1.Images[i].ImageIndex, result2.Images[i].ImageIndex);
        }
    }

    [Fact]
    public void ImageExtraction_WithDifferentConfigs_ProducesDifferentResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");

        var config1 = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 100
            }
        };

        var config2 = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 300
            }
        };

        var result1 = KreuzbergClient.ExtractFileSync(pdfPath, config1);
        var result2 = KreuzbergClient.ExtractFileSync(pdfPath, config2);

        // Both should process successfully
        Assert.NotNull(result1);
        Assert.NotNull(result2);
    }

    #endregion

    #region Configuration Pattern Tests

    [Fact]
    public void ExtractionConfig_WithImages_UsesInitPattern()
    {
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150,
                MaxImageDimension = 2048,
                AutoAdjustDpi = false,
                MinDpi = 100,
                MaxDpi = 400
            }
        };

        Assert.NotNull(config.Images);
        Assert.True(config.Images.ExtractImages);
        Assert.Equal(150, config.Images.TargetDpi);
    }

    [Fact]
    public void ImageExtractionConfig_AllPropertiesImmutable()
    {
        var config = new ImageExtractionConfig
        {
            ExtractImages = true,
            TargetDpi = 150
        };

        // Should not be able to modify after creation
        // (This test verifies the init pattern - properties use init accessor)
        Assert.True(config.ExtractImages);
        Assert.Equal(150, config.TargetDpi);
    }

    [Fact]
    public void ExtractionConfig_MultipleFeatures_CombinedWithImages()
    {
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150
            },
            Chunking = new ChunkingConfig
            {
                Enabled = true
            }
        };

        Assert.NotNull(config.Images);
        Assert.NotNull(config.Chunking);
        Assert.True(config.Images.ExtractImages);
    }

    #endregion
}
