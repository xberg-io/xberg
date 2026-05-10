```csharp title="C#"
using Kreuzberg;

var result = KreuzbergLib.ExtractFileSync("document.pdf", new ExtractionConfig());
Console.WriteLine(result.Content);
```
