```csharp title="C#"
using Kreuzberg;

var data = await File.ReadAllBytesAsync("document.pdf");
var result = KreuzbergLib.ExtractBytesSync(data, "application/pdf");

Console.WriteLine(result.Content);
Console.WriteLine(result.MimeType);
```
