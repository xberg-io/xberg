using Xberg;
using System.Linq;

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
var basicYakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.0f,
        Language = "en",
        YakeParams = null,
        RakeParams = null,
    }
};

var output = await XbergConverter.ExtractAsync(ExtractInput.FromUri("document.pdf"), basicYakeConfig);
var result = output.Results[0];
Console.WriteLine($"Keywords: {string.Join(", ", result.ExtractedKeywords?.Select(k => k.Text) ?? Enumerable.Empty<string>())}");

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
var advancedYakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 15,
        MinScore = 0.1f,
        Language = "en",
        YakeParams = new YakeParams
        {
            WindowSize = 1,
        },
        RakeParams = null,
    }
};

output = await XbergConverter.ExtractAsync(ExtractInput.FromUri("document.pdf"), advancedYakeConfig);
result = output.Results[0];
Console.WriteLine($"Keywords: {string.Join(", ", result.ExtractedKeywords?.Select(k => k.Text) ?? Enumerable.Empty<string>())}");

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
var rakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Rake,
        MaxKeywords = 10,
        MinScore = 5.0f,
        Language = "en",
        YakeParams = null,
        RakeParams = new RakeParams
        {
            MinWordLength = 1,
            MaxWordsPerPhrase = 3,
        },
    }
};

output = await XbergConverter.ExtractAsync(ExtractInput.FromUri("document.pdf"), rakeConfig);
result = output.Results[0];
Console.WriteLine($"Keywords: {string.Join(", ", result.ExtractedKeywords?.Select(k => k.Text) ?? Enumerable.Empty<string>())}");
