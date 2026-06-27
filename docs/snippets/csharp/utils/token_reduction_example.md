```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionOptions
    {
        Mode = "moderate",
        PreserveImportantWords = true,
    },
};

var result = await XbergLib.Extract("verbose_document.pdf", null, config);

var additional = result.Metadata.Additional;
var original = additional.TryGetValue("original_token_count", out var o) ? o : 0;
var reduced = additional.TryGetValue("token_count", out var r) ? r : 0;
var ratio = additional.TryGetValue("token_reduction_ratio", out var rr) ? rr : 0.0;

Console.WriteLine($"Reduced from {original} to {reduced} tokens");
Console.WriteLine($"Reduction: {Convert.ToDouble(ratio) * 100:F1}%");
```
