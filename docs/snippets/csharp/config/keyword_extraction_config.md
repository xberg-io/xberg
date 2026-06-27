```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.1f,
        NgramRange = [1, 3],
        Language = "en"
    }
};

var result = await XbergLib.Extract("document.pdf", null, config);
if (result.Keywords != null)
{
    Console.WriteLine($"Keywords: {string.Join(", ", result.Keywords)}");
}
```
