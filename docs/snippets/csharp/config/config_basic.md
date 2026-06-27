```csharp title="C#"
using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine(result.Content);
```
