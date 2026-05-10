```csharp title="C#"
using Kreuzberg;

var result = await KreuzbergLib.ExtractFileAsync("document.pdf");

Console.WriteLine(result.Content);
Console.WriteLine(result.MimeType);
```
