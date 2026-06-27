using Xberg;
using Xberg.Keywords;

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
var basicYakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.0f,
        NgramRange = (1, 3),
        Language = "en",
        YakeParams = null,
        RakeParams = null,
    }
};

var result = XbergLib.ExtractSync("document.pdf", basicYakeConfig);
Console.WriteLine($"Keywords: {string.Join(", ", result.Keywords)}");

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
var advancedYakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 15,
        MinScore = 0.1f,
        NgramRange = (1, 2),
        Language = "en",
        YakeParams = new YakeParams
        {
            WindowSize = 1,
        },
        RakeParams = null,
    }
};

result = XbergLib.ExtractSync("document.pdf", advancedYakeConfig);
Console.WriteLine($"Keywords: {string.Join(", ", result.Keywords)}");

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
var rakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Rake,
        MaxKeywords = 10,
        MinScore = 5.0f,
        NgramRange = (1, 3),
        Language = "en",
        YakeParams = null,
        RakeParams = new RakeParams
        {
            MinWordLength = 1,
            MaxWordsPerPhrase = 3,
        },
    }
};

result = XbergLib.ExtractSync("document.pdf", rakeConfig);
Console.WriteLine($"Keywords: {string.Join(", ", result.Keywords)}");
