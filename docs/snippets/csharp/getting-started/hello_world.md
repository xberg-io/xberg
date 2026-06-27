```csharp title="C#"
using Xberg;

var result = XbergLib.Extract("document.pdf", new ExtractionConfig());
Console.WriteLine(result.Content);
```
