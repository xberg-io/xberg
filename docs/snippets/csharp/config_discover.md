```csharp title="C#"
using Xberg;

var config = new ExtractionConfig();
var result = await XbergLib.ExtractAsync("document.pdf", config);

Console.WriteLine(result.Content[..Math.Min(100, result.Content.Length)]);
Console.WriteLine($"Total length: {result.Content.Length}");
```
