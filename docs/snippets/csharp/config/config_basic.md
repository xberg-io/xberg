```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine(result.Content);
```
