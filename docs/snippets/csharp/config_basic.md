```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config);
Console.WriteLine(result.Content);
```
