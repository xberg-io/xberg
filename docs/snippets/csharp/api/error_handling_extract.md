```csharp title="C#"
using Xberg;

try
{
    var data = File.ReadAllBytes("document.unsupported");
    var result = XbergLib.ExtractSync(data, "application/x-custom", null);
    Console.WriteLine(result.Content);
}
catch (XbergException ex) when (ex.Code == 1)
{
    Console.WriteLine("Validation error: Invalid MIME type");
}
catch (XbergException ex) when (ex.Code == 2)
{
    Console.WriteLine("Format error: MIME type not supported");
}
catch (XbergException ex)
{
    Console.WriteLine($"Extraction failed with error {ex.Code}: {ex.Message}");
}
```
