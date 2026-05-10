```csharp title="C#"
using Kreuzberg;

var data = await File.ReadAllBytesAsync("document.pdf");
var config = new ExtractionConfig { OutputFormat = OutputFormat.Text };
var result = await KreuzbergLib.ExtractBytes(data, "application/pdf", config);

Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
```
