```csharp title="C#"
using Xberg;

var config = ExtractionConfig.Discover() ?? new ExtractionConfig();

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine(result.Content);
```
