```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig();
var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
Console.WriteLine(result.Content);
```
