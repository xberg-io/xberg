```csharp title="C#"
using Xberg;

try
{
    var result = XbergLib.ExtractSync("missing.pdf");
    Console.WriteLine(result.Content);
}
catch (XbergValidationException ex)
{
    Console.Error.WriteLine($"Validation error: {ex.Message}");
}
catch (XbergIOException ex)
{
    Console.Error.WriteLine($"IO error: {ex.Message}");
    throw;
}
catch (XbergException ex)
{
    Console.Error.WriteLine($"Extraction failed: {ex.Message}");
    throw;
}
```
