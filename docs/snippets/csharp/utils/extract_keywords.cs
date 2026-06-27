```csharp title="extract_keywords.cs"
using Xberg;
using System;
using System.Collections.Generic;
using System.Linq;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.3f
    }
};

var output = await XbergConverter.ExtractAsync(ExtractInput.FromUri("research_paper.pdf"), config);
var result = output.Results[0];

Console.WriteLine("Extracted Keywords:");
if (result.ExtractedKeywords != null)
{
    foreach (var keyword in result.ExtractedKeywords.OrderByDescending(k => k.Score))
    {
        Console.WriteLine($"  - {keyword.Text}: {keyword.Score:F3}");
    }
}
else
{
    Console.WriteLine("  (No keywords extracted)");
}

var rakeConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Rake,
        MaxKeywords = 15,
        MinScore = 0.2f
    }
};

var rakeOutput = await XbergConverter.ExtractAsync(ExtractInput.FromUri("document.pdf"), rakeConfig);
var rakeResult = rakeOutput.Results[0];

Console.WriteLine("\nRAKE Keywords:");
if (rakeResult.ExtractedKeywords != null)
{
    var topKeywords = rakeResult.ExtractedKeywords
        .OrderByDescending(k => k.Score)
        .Take(10)
        .ToList();

    foreach (var keyword in topKeywords)
    {
        Console.WriteLine($"  - {keyword.Text}: {keyword.Score:F3}");
    }
}

Console.WriteLine($"\nKeyword Extraction Summary:");
Console.WriteLine($"  - Algorithm: YAKE");
Console.WriteLine($"  - Total Keywords: {result.ExtractedKeywords?.Count ?? 0}");
Console.WriteLine($"  - Top Keyword: {result.ExtractedKeywords?.FirstOrDefault()?.Text}");
```
