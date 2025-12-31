using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive immutability validation tests for C# configuration classes.
/// Validates that the { get; init; } pattern is properly enforced across all config types.
/// Tests actual immutability by verifying init-only accessors prevent post-initialization mutations.
/// </summary>
public class ImmutabilityTests
{
    public ImmutabilityTests()
    {
        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region Helper Methods

    /// <summary>
    /// Helper: Check if a property has the IsExternalInit modifier on its set method.
    /// This modifier is what makes init-only accessors function correctly and prevent mutations.
    /// </summary>
    private static bool HasIsExternalInitModifier(PropertyInfo property)
    {
        if (property.SetMethod == null)
            return false;

        return property.SetMethod.ReturnParameter?
            .GetRequiredCustomModifiers()
            .Any(m => m.Name == "IsExternalInit") ?? false;
    }

    /// <summary>
    /// Helper: Verify a config type enforces immutability across all settable properties.
    /// </summary>
    private void VerifyConfigTypeIsImmutable(Type configType)
    {
        var properties = configType.GetProperties(BindingFlags.Public | BindingFlags.Instance)
            .Where(p => p.SetMethod != null)
            .ToList();

        // All config types should have at least one settable property
        Assert.True(properties.Count > 0, $"{configType.Name} should have at least one settable property");

        foreach (var prop in properties)
        {
            Assert.True(HasIsExternalInitModifier(prop),
                $"{configType.Name}.{prop.Name} must have init-only accessor (IsExternalInit modifier missing)");
        }
    }

    #endregion

    #region ExtractionConfig Immutability Tests

    /// <summary>
    /// Test: Validate ExtractionConfig enforces immutability via init-only properties.
    /// Verifies that all properties use init-only accessors that prevent post-initialization mutation.
    /// </summary>
    [Fact]
    public void ExtractionConfigIsImmutable_AllPropertiesAreInitOnly()
    {
        var config = new ExtractionConfig
        {
            UseCache = true,
            EnableQualityProcessing = false,
            ForceOcr = true,
            MaxConcurrentExtractions = 5
        };

        // Verify values were set during initialization
        Assert.Equal(true, config.UseCache);
        Assert.Equal(false, config.EnableQualityProcessing);
        Assert.Equal(true, config.ForceOcr);
        Assert.Equal(5, config.MaxConcurrentExtractions);

        // Verify all writable properties have init-only accessors
        VerifyConfigTypeIsImmutable(typeof(ExtractionConfig));
    }

    /// <summary>
    /// Test: Verify post-initialization mutation attempts are prevented by init-only modifier.
    /// The IsExternalInit modifier prevents property assignment outside initialization.
    /// </summary>
    [Fact]
    public void ExtractionConfigPreventsPostInitializationMutation()
    {
        var config = new ExtractionConfig { UseCache = true };
        var useCacheProperty = typeof(ExtractionConfig).GetProperty("UseCache");

        Assert.NotNull(useCacheProperty);
        Assert.True(HasIsExternalInitModifier(useCacheProperty),
            "UseCache property must have IsExternalInit modifier to prevent post-initialization mutation");

        // Verify we can read the value (public getter)
        var value = useCacheProperty.GetValue(config);
        Assert.Equal(true, value);
    }

    #endregion

    #region OcrConfig Immutability Tests

    /// <summary>
    /// Test: Validate OcrConfig implements immutability pattern.
    /// Ensures nested config objects also enforce init-only properties.
    /// </summary>
    [Fact]
    public void OcrConfigIsImmutable_NestedConfigEnforcesPattern()
    {
        var config = new OcrConfig
        {
            Backend = "tesseract",
            Language = "eng"
        };

        Assert.Equal("tesseract", config.Backend);
        Assert.Equal("eng", config.Language);

        // Verify all properties have init-only accessors
        VerifyConfigTypeIsImmutable(typeof(OcrConfig));
    }

    #endregion

    #region TesseractConfig Immutability Tests

    /// <summary>
    /// Test: Validate TesseractConfig enforces immutability with complex properties.
    /// Verifies init-only pattern on properties with various types (string, int, double).
    /// </summary>
    [Fact]
    public void TesseractConfigIsImmutable_ComplexPropertiesAreInitOnly()
    {
        var config = new TesseractConfig
        {
            Language = "eng",
            Psm = 3,
            MinConfidence = 0.5,
        };

        Assert.Equal("eng", config.Language);
        Assert.Equal(3, config.Psm);
        Assert.Equal(0.5, config.MinConfidence);

        // Verify all properties with setters are init-only
        VerifyConfigTypeIsImmutable(typeof(TesseractConfig));
    }

    #endregion

    #region ImagePreprocessingConfig Immutability Tests

    /// <summary>
    /// Test: Validate ImagePreprocessingConfig enforces immutability.
    /// </summary>
    [Fact]
    public void ImagePreprocessingConfigIsImmutable_AllPropertiesInitOnly()
    {
        var config = new ImagePreprocessingConfig
        {
            TargetDpi = 300,
            AutoRotate = true,
            Deskew = true,
            Denoise = true,
            ContrastEnhance = true
        };

        Assert.Equal(300, config.TargetDpi);
        Assert.True(config.AutoRotate);

        VerifyConfigTypeIsImmutable(typeof(ImagePreprocessingConfig));
    }

    #endregion

    #region ChunkingConfig Immutability Tests

    /// <summary>
    /// Test: Validate ChunkingConfig with EmbeddingConfig is immutable.
    /// Verifies init-only pattern on properties containing nested config objects.
    /// </summary>
    [Fact]
    public void ChunkingConfigIsImmutable_WithEmbeddingConfig()
    {
        var embeddingConfig = new EmbeddingConfig
        {
            Model = "text-embedding-ada-002",
            Dimensions = 1536,
            BatchSize = 32,
            Normalize = true
        };

        var config = new ChunkingConfig
        {
            MaxChars = 1024,
            MaxOverlap = 128,
            ChunkSize = 512,
            ChunkOverlap = 64,
            Preset = "default",
            Enabled = true,
            Embedding = embeddingConfig
        };

        Assert.Equal(1024, config.MaxChars);
        Assert.NotNull(config.Embedding);
        Assert.True(config.Enabled);

        // Verify embedding property is init-only
        var embeddingProp = typeof(ChunkingConfig).GetProperty("Embedding");
        Assert.NotNull(embeddingProp);
        Assert.True(HasIsExternalInitModifier(embeddingProp),
            "Embedding property must have init-only accessor");

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(ChunkingConfig));
    }

    #endregion

    #region ImageExtractionConfig Immutability Tests

    /// <summary>
    /// Test: Validate ImageExtractionConfig enforces immutability.
    /// Verifies init-only pattern on image-specific properties.
    /// </summary>
    [Fact]
    public void ImageExtractionConfigIsImmutable_AllPropertiesInitOnly()
    {
        var config = new ImageExtractionConfig
        {
            ExtractImages = true,
            TargetDpi = 300,
            MaxImageDimension = 2048,
            AutoAdjustDpi = true,
            MinDpi = 72,
            MaxDpi = 600
        };

        Assert.True(config.ExtractImages);
        Assert.Equal(300, config.TargetDpi);

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(ImageExtractionConfig));
    }

    #endregion

    #region PdfConfig Immutability Tests

    /// <summary>
    /// Test: Validate PdfConfig with nested configs enforces immutability.
    /// Verifies init-only pattern on nested FontConfig and HierarchyConfig properties.
    /// </summary>
    [Fact]
    public void PdfConfigIsImmutable_WithNestedConfigs()
    {
        var fontConfig = new FontConfig
        {
            FontFallbackEnabled = true,
            FontDir = "/usr/share/fonts"
        };

        var hierarchyConfig = new HierarchyConfig
        {
            Enabled = true,
            KClusters = 5,
            IncludeBbox = true,
            OcrCoverageThreshold = 0.8f
        };

        var config = new PdfConfig
        {
            ExtractImages = true,
            ExtractMetadata = true,
            Passwords = new List<string> { "password1", "password2" },
            FontConfig = fontConfig,
            Hierarchy = hierarchyConfig
        };

        Assert.NotNull(config.FontConfig);
        Assert.NotNull(config.Hierarchy);
        Assert.True(config.FontConfig.FontFallbackEnabled);

        // Verify nested config properties are init-only
        var fontConfigProp = typeof(PdfConfig).GetProperty("FontConfig");
        Assert.NotNull(fontConfigProp);
        Assert.True(HasIsExternalInitModifier(fontConfigProp),
            "FontConfig property must have init-only accessor");

        // Verify all properties in PdfConfig and nested types are init-only
        VerifyConfigTypeIsImmutable(typeof(PdfConfig));
        VerifyConfigTypeIsImmutable(typeof(FontConfig));
        VerifyConfigTypeIsImmutable(typeof(HierarchyConfig));
    }

    #endregion

    #region PageConfig Immutability Tests

    /// <summary>
    /// Test: Validate PageConfig enforces immutability.
    /// Verifies init-only pattern on page-related configuration.
    /// </summary>
    [Fact]
    public void PageConfigIsImmutable_PagePropertiesInitOnly()
    {
        var config = new PageConfig
        {
            ExtractPages = true,
            InsertPageMarkers = true,
            MarkerFormat = "[PAGE_{0}]"
        };

        Assert.True(config.ExtractPages);
        Assert.Equal("[PAGE_{0}]", config.MarkerFormat);

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(PageConfig));
    }

    #endregion

    #region TokenReductionConfig Immutability Tests

    /// <summary>
    /// Test: Validate TokenReductionConfig enforces immutability.
    /// </summary>
    [Fact]
    public void TokenReductionConfigIsImmutable()
    {
        var config = new TokenReductionConfig
        {
            Mode = "balanced",
            PreserveImportantWords = true
        };

        Assert.Equal("balanced", config.Mode);
        Assert.True(config.PreserveImportantWords);
        VerifyConfigTypeIsImmutable(typeof(TokenReductionConfig));
    }

    #endregion

    #region LanguageDetectionConfig Immutability Tests

    /// <summary>
    /// Test: Validate LanguageDetectionConfig enforces immutability.
    /// </summary>
    [Fact]
    public void LanguageDetectionConfigIsImmutable()
    {
        var config = new LanguageDetectionConfig
        {
            Enabled = true
        };

        Assert.True(config.Enabled);
        VerifyConfigTypeIsImmutable(typeof(LanguageDetectionConfig));
    }

    #endregion

    #region PostProcessorConfig Immutability Tests

    /// <summary>
    /// Test: Validate PostProcessorConfig enforces immutability.
    /// </summary>
    [Fact]
    public void PostProcessorConfigIsImmutable()
    {
        var config = new PostProcessorConfig();

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(PostProcessorConfig));
    }

    #endregion

    #region HtmlConversionOptions Immutability Tests

    /// <summary>
    /// Test: Validate HtmlConversionOptions enforces immutability.
    /// </summary>
    [Fact]
    public void HtmlConversionOptionsIsImmutable()
    {
        var config = new HtmlConversionOptions();

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(HtmlConversionOptions));
    }

    #endregion

    #region HtmlPreprocessingOptions Immutability Tests

    /// <summary>
    /// Test: Validate HtmlPreprocessingOptions enforces immutability.
    /// </summary>
    [Fact]
    public void HtmlPreprocessingOptionsIsImmutable()
    {
        var config = new HtmlPreprocessingOptions();

        // Verify all properties are init-only
        VerifyConfigTypeIsImmutable(typeof(HtmlPreprocessingOptions));
    }

    #endregion

    #region KeywordConfig Immutability Tests

    /// <summary>
    /// Test: Validate KeywordConfig enforces immutability.
    /// </summary>
    [Fact]
    public void KeywordConfigIsImmutable()
    {
        var config = new KeywordConfig
        {
            Algorithm = "yake",
            MaxKeywords = 10,
            Language = "en"
        };

        Assert.Equal("yake", config.Algorithm);
        Assert.Equal(10, config.MaxKeywords);
        Assert.Equal("en", config.Language);
        VerifyConfigTypeIsImmutable(typeof(KeywordConfig));
    }

    #endregion

    #region Complex Nested Configuration Immutability Tests

    /// <summary>
    /// Test: Validate complete nested config hierarchy maintains immutability.
    /// Creates a complex extraction config with multiple nested configs and verifies immutability.
    /// </summary>
    [Fact]
    public void NestedConfigHierarchy_MaintainsImmutabilityAcrossAllLevels()
    {
        var config = new ExtractionConfig
        {
            UseCache = true,
            Ocr = new OcrConfig
            {
                Backend = "tesseract",
                Language = "eng",
                TesseractConfig = new TesseractConfig
                {
                    Language = "eng",
                    Psm = 3,
                    MinConfidence = 0.5,
                    Preprocessing = new ImagePreprocessingConfig
                    {
                        TargetDpi = 300,
                        AutoRotate = true,
                        Deskew = true,
                        Denoise = true,
                        ContrastEnhance = true
                    }
                }
            },
            Chunking = new ChunkingConfig
            {
                MaxChars = 1024,
                MaxOverlap = 128,
                Preset = "default"
            },
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 300
            },
            PdfOptions = new PdfConfig
            {
                ExtractImages = true,
                ExtractMetadata = true,
                FontConfig = new FontConfig
                {
                    FontFallbackEnabled = true,
                    FontDir = "/fonts"
                },
                Hierarchy = new HierarchyConfig
                {
                    Enabled = true,
                    KClusters = 5
                }
            },
            Pages = new PageConfig
            {
                ExtractPages = true,
                InsertPageMarkers = true,
                MarkerFormat = "[PAGE_{0}]"
            }
        };

        // Verify root level immutability
        Assert.True(config.UseCache);

        // Verify nested level 1 immutability
        Assert.NotNull(config.Ocr);
        Assert.Equal("tesseract", config.Ocr.Backend);

        // Verify nested level 2 immutability
        Assert.NotNull(config.Ocr.TesseractConfig);
        Assert.Equal("eng", config.Ocr.TesseractConfig.Language);

        // Verify nested level 3 immutability
        Assert.NotNull(config.Ocr.TesseractConfig.Preprocessing);
        Assert.Equal(300, config.Ocr.TesseractConfig.Preprocessing.TargetDpi);

        // Verify all nested configs at each level have init-only properties
        VerifyConfigTypeIsImmutable(typeof(ExtractionConfig));
        VerifyConfigTypeIsImmutable(typeof(OcrConfig));
        VerifyConfigTypeIsImmutable(typeof(TesseractConfig));
        VerifyConfigTypeIsImmutable(typeof(ImagePreprocessingConfig));
        VerifyConfigTypeIsImmutable(typeof(ChunkingConfig));
        VerifyConfigTypeIsImmutable(typeof(ImageExtractionConfig));
        VerifyConfigTypeIsImmutable(typeof(PdfConfig));
        VerifyConfigTypeIsImmutable(typeof(FontConfig));
        VerifyConfigTypeIsImmutable(typeof(HierarchyConfig));
        VerifyConfigTypeIsImmutable(typeof(PageConfig));
    }

    #endregion

    #region Object Initializer Syntax Tests

    /// <summary>
    /// Test: Verify object initializer syntax works correctly with init-only properties.
    /// Validates that properties can be set during initialization via object initializer syntax.
    /// </summary>
    [Fact]
    public void ObjectInitializerSyntax_WorksWithInitOnlyProperties()
    {
        // Using object initializer syntax with init-only properties
        var config = new ExtractionConfig
        {
            UseCache = true,
            EnableQualityProcessing = false,
            ForceOcr = true,
            MaxConcurrentExtractions = 10,
            Ocr = new OcrConfig
            {
                Backend = "tesseract",
                Language = "eng"
            },
            Chunking = new ChunkingConfig
            {
                MaxChars = 1024,
                MaxOverlap = 128
            }
        };

        // Verify all values were set correctly
        Assert.True(config.UseCache);
        Assert.False(config.EnableQualityProcessing);
        Assert.True(config.ForceOcr);
        Assert.Equal(10, config.MaxConcurrentExtractions);
        Assert.NotNull(config.Ocr);
        Assert.Equal("tesseract", config.Ocr.Backend);
        Assert.NotNull(config.Chunking);
        Assert.Equal(1024, config.Chunking.MaxChars);
    }

    #endregion

    #region Default Constructor Tests

    /// <summary>
    /// Test: Verify default constructor initialization with init properties.
    /// Validates that configs can be created with default values without object initializer.
    /// </summary>
    [Fact]
    public void DefaultConstructorInitialization_CreatesConfigWithDefaults()
    {
        var config = new ExtractionConfig();

        // All nullable properties should be null by default
        Assert.Null(config.UseCache);
        Assert.Null(config.EnableQualityProcessing);
        Assert.Null(config.ForceOcr);
        Assert.Null(config.Ocr);
        Assert.Null(config.Chunking);

        // Should still be readable
        var _ = config.UseCache;
        // If no exception, reading properties works correctly
    }

    #endregion

    #region All Config Types Comprehensive Coverage

    /// <summary>
    /// Test: Verify all config types in the Kreuzberg namespace enforce immutability.
    /// Comprehensively checks all config classes for init-only pattern.
    /// </summary>
    [Fact]
    public void AllConfigTypes_EnforceInitOnlyPattern()
    {
        var configTypes = new[]
        {
            typeof(ExtractionConfig),
            typeof(OcrConfig),
            typeof(TesseractConfig),
            typeof(ImagePreprocessingConfig),
            typeof(ChunkingConfig),
            typeof(ImageExtractionConfig),
            typeof(FontConfig),
            typeof(HierarchyConfig),
            typeof(PdfConfig),
            typeof(TokenReductionConfig),
            typeof(LanguageDetectionConfig),
            typeof(PostProcessorConfig),
            typeof(HtmlConversionOptions),
            typeof(HtmlPreprocessingOptions),
            typeof(KeywordConfig),
            typeof(PageConfig)
        };

        foreach (var configType in configTypes)
        {
            var properties = configType.GetProperties(BindingFlags.Public | BindingFlags.Instance)
                .Where(p => p.SetMethod != null && !p.Name.StartsWith("_"))
                .ToList();

            // If type has any settable properties, verify they're all init-only
            if (properties.Count > 0)
            {
                foreach (var prop in properties)
                {
                    Assert.True(HasIsExternalInitModifier(prop),
                        $"{configType.Name}.{prop.Name} must have init-only accessor");
                }
            }
        }
    }

    #endregion
}
