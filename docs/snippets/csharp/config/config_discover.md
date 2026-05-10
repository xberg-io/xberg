```csharp title="C#"
using Kreuzberg;

var config = ExtractionConfig.Discover() ?? new ExtractionConfig();

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine(result.Content);
```
