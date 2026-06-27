```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.YAKE,
        MaxKeywords = 10,
        MinScore = 0.3f,
    },
};

var result = await XbergLib.Extract("research_paper.pdf", null, config);

foreach (var keyword in result.ExtractedKeywords ?? new List<Keyword>())
{
    Console.WriteLine($"{keyword.Text}: {keyword.Score:F3}");
}
```
