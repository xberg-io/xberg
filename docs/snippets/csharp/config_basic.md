```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine(result.Content);
```
