```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig { OutputFormat = OutputFormat.Text };
var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);

Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
```
