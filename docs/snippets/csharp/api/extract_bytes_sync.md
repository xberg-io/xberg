```csharp title="C#"
using Kreuzberg;

var data = File.ReadAllBytes("document.pdf");
var config = new ExtractionConfig { OutputFormat = OutputFormat.Text };
var result = KreuzbergLib.ExtractBytesSync(data, "application/pdf", config);

Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
```
