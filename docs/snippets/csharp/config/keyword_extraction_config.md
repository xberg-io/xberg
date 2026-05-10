```csharp title="C#"
using Kreuzberg;

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

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
if (result.Keywords != null)
{
    Console.WriteLine($"Keywords: {string.Join(", ", result.Keywords)}");
}
```
