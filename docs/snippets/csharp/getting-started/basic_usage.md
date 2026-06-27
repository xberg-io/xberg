```csharp title="C#"
using Xberg;

var config = new ExtractionConfig();
var result = XbergLib.ExtractSync("document.pdf", config);
Console.WriteLine(result.Content);
```
