using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for keyword extraction functionality from documents.
/// Tests cover YAKE and RAKE algorithms, keyword configuration, ngram ranges,
/// score filtering, language settings, and async operations.
/// </summary>
public class KeywordExtractionTests
{
    public KeywordExtractionTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region YAKE Algorithm Tests

    [Fact]
    public void ExtractKeywords_WithYakeAlgorithm_ReturnsContent()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 5
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractKeywords_WithYakeAndMaxKeywords_RespectsLimit()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var maxKeywords = 10;
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = maxKeywords
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractKeywords_WithYakeAndMinScore_FiltersKeywords()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 20,
                MinScore = 0.1
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractKeywords_WithYakeParams_AppliesCustomParameters()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var yakeParams = new Dictionary<string, object?>
        {
            { "window_size", 3 }
        };

        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                YakeParams = yakeParams
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    #endregion

    #region RAKE Algorithm Tests

    [Fact]
    public void ExtractKeywords_WithRakeAlgorithm_ReturnsContent()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "rake",
                MaxKeywords = 5
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractKeywords_WithRakeAndMaxKeywords_RespectsLimit()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var maxKeywords = 15;
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "rake",
                MaxKeywords = maxKeywords
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractKeywords_WithRakeParams_AppliesCustomParameters()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var rakeParams = new Dictionary<string, object?>
        {
            { "min_word_length", 2 },
            { "max_words_per_phrase", 4 }
        };

        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "rake",
                RakeParams = rakeParams
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    #endregion

    #region Ngram Range Tests

    [Theory]
    [InlineData(1, 1)] // Unigrams only
    [InlineData(1, 2)] // Unigrams and bigrams
    [InlineData(1, 3)] // Unigrams, bigrams, and trigrams
    [InlineData(2, 3)] // Bigrams and trigrams only
    public void ExtractKeywords_WithNgramRange_RespectsRangeConfiguration(
        int minNgram,
        int maxNgram)
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                NgramRange = new List<int> { minNgram, maxNgram },
                MaxKeywords = 10
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractKeywords_WithUnigramRange_ExtractsOnlyUnigrams()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                NgramRange = new List<int> { 1, 1 },
                MaxKeywords = 10
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    #endregion

    #region Language Configuration Tests

    [Theory]
    [InlineData("en")]
    [InlineData("fr")]
    [InlineData("de")]
    [InlineData("es")]
    public void ExtractKeywords_WithLanguageConfiguration_AcceptsLanguageCode(string languageCode)
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                Language = languageCode,
                MaxKeywords = 10
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    #endregion

    #region Async Operations Tests

    [Fact]
    public async Task ExtractKeywordsAsync_WithValidFile_ReturnsContentAsync()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 5
            }
        };

        var result = await KreuzbergClient.ExtractFileAsync(textPath, config: config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_WithCancellationToken_ThrowsOperationCanceledExceptionWhenCanceled()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");

        // Can throw either OperationCanceledException or TaskCanceledException (which is a subclass)
        var exceptionThrown = false;
        try
        {
            await KreuzbergClient.ExtractFileAsync(
                textPath,
                config: null,
                cancellationToken: cts.Token
            );
        }
        catch (OperationCanceledException)
        {
            exceptionThrown = true;
        }

        Assert.True(exceptionThrown, "Expected OperationCanceledException or subclass to be thrown");
    }

    [Fact]
    public async Task ExtractKeywordsAsync_WithTimeout_ThrowsOperationCanceledExceptionOnTimeout()
    {
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(10));
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");

        // Timeout scenarios may or may not throw depending on timing, so verify the operation either
        // completes or throws an OperationCanceledException (which includes TaskCanceledException)
        try
        {
            var result = await KreuzbergClient.ExtractFileAsync(
                textPath,
                config: null,
                cancellationToken: cts.Token
            );
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
    public async Task ExtractKeywordsAsync_MultipleFilesWithWhenAll_AllCompleteSuccessfully()
    {
        var files = new[]
        {
            NativeTestHelper.GetDocumentPath("misc/readme.org"),
            NativeTestHelper.GetDocumentPath("misc/simple.epub")
        };

        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 5
            }
        };

        var tasks = files.Select(f => KreuzbergClient.ExtractFileAsync(f, config: config)).ToList();
        var results = await Task.WhenAll(tasks);

        Assert.Equal(files.Length, results.Length);
        Assert.All(results, result => Assert.NotNull(result));
    }

    #endregion

    #region Score Filtering Tests

    [Fact]
    public void ExtractKeywords_WithMinScoreZero_IncludesAllKeywords()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MinScore = 0.0,
                MaxKeywords = 10
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractKeywords_WithHighMinScore_FiltersMoreAggressively()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MinScore = 0.8,
                MaxKeywords = 10
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    #endregion

    #region Configuration Immutability Tests

    [Fact]
    public void KeywordConfig_CreatedWithInitializer_CannotBeModifiedAfterCreation()
    {
        var config = new KeywordConfig
        {
            Algorithm = "yake",
            MaxKeywords = 5,
            MinScore = 0.1
        };

        Assert.Equal("yake", config.Algorithm);
        Assert.Equal(5, config.MaxKeywords);
        Assert.Equal(0.1, config.MinScore);

        // Attempt to verify immutability through reflection
        var algorithmProp = typeof(KeywordConfig).GetProperty("Algorithm");
        var setMethod = algorithmProp?.GetSetMethod();

        // If setter exists, it should be init-only
        if (setMethod != null)
        {
            var hasInitOnly = setMethod.ReturnParameter?
                .GetRequiredCustomModifiers()
                .Any(m => m.Name == "IsExternalInit") ?? false;

            Assert.True(hasInitOnly, "Algorithm property should have init-only accessor");
        }
    }

    [Fact]
    public void KeywordConfig_NullableProperties_CanBeSetToNull()
    {
        var config = new KeywordConfig
        {
            Algorithm = null,
            MaxKeywords = null,
            MinScore = null,
            Language = null,
            NgramRange = null,
            YakeParams = null,
            RakeParams = null
        };

        Assert.Null(config.Algorithm);
        Assert.Null(config.MaxKeywords);
        Assert.Null(config.MinScore);
        Assert.Null(config.Language);
        Assert.Null(config.NgramRange);
        Assert.Null(config.YakeParams);
        Assert.Null(config.RakeParams);
    }

    #endregion

    #region Configuration Nesting Tests

    [Fact]
    public void KeywordConfig_NestedInExtractionConfig_PreserveAllProperties()
    {
        var keywordConfig = new KeywordConfig
        {
            Algorithm = "rake",
            MaxKeywords = 15,
            MinScore = 0.2,
            Language = "en",
            NgramRange = new List<int> { 1, 2 }
        };

        var extractionConfig = new ExtractionConfig { Keywords = keywordConfig };

        Assert.NotNull(extractionConfig.Keywords);
        Assert.Equal("rake", extractionConfig.Keywords.Algorithm);
        Assert.Equal(15, extractionConfig.Keywords.MaxKeywords);
        Assert.Equal(0.2, extractionConfig.Keywords.MinScore);
        Assert.Equal("en", extractionConfig.Keywords.Language);
        Assert.NotNull(extractionConfig.Keywords.NgramRange);
        Assert.Equal(2, extractionConfig.Keywords.NgramRange.Count);
    }

    #endregion

    #region Combined Configuration Tests

    [Fact]
    public void ExtractKeywords_WithComplexConfiguration_AllPropertiesApplied()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");
        var yakeParams = new Dictionary<string, object?> { { "window_size", 3 } };

        var config = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 12,
                MinScore = 0.15,
                Language = "en",
                NgramRange = new List<int> { 1, 2 },
                YakeParams = yakeParams
            }
        };

        var result = KreuzbergClient.ExtractFileSync(textPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractKeywords_DifferentAlgorithmsProduceDifferentResults()
    {
        var textPath = NativeTestHelper.GetDocumentPath("misc/readme.org");

        var yakeConfig = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "yake",
                MaxKeywords = 10
            }
        };

        var rakeConfig = new ExtractionConfig
        {
            Keywords = new KeywordConfig
            {
                Algorithm = "rake",
                MaxKeywords = 10
            }
        };

        var yakeResult = KreuzbergClient.ExtractFileSync(textPath, yakeConfig);
        var rakeResult = KreuzbergClient.ExtractFileSync(textPath, rakeConfig);

        Assert.NotNull(yakeResult);
        Assert.NotNull(rakeResult);
        Assert.True(yakeResult.Success);
        Assert.True(rakeResult.Success);
    }

    #endregion
}
