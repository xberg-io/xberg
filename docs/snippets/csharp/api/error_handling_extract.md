```csharp title="C#"
using Kreuzberg;

try
{
    var data = File.ReadAllBytes("document.unsupported");
    var result = KreuzbergLib.ExtractBytesSync(data, "application/x-custom", null);
    Console.WriteLine(result.Content);
}
catch (KreuzbergException ex) when (ex.Code == 1)
{
    Console.WriteLine("Validation error: Invalid MIME type");
}
catch (KreuzbergException ex) when (ex.Code == 2)
{
    Console.WriteLine("Format error: MIME type not supported");
}
catch (KreuzbergException ex)
{
    Console.WriteLine($"Extraction failed with error {ex.Code}: {ex.Message}");
}
```
