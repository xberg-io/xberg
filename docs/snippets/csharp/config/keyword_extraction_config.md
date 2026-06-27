```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.1f,
        Language = "en"
    }
};

var output = await XbergConverter.ExtractAsync(ExtractInput.FromUri("document.pdf"), config);
var result = output.Results[0];
if (result.ExtractedKeywords != null)
{
    Console.WriteLine($"Keywords: {string.Join(", ", result.ExtractedKeywords.Select(k => k.Text))}");
}
```
