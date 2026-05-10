```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig { OutputFormat = OutputFormat.Text };
var result = KreuzbergLib.ExtractFileSync("document.pdf", null, config);

Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
```
