```csharp title="C#"
using Kreuzberg;

try
{
    var result = KreuzbergLib.ExtractFileSync("nonexistent.pdf", null, null);
    Console.WriteLine(result.Content);
}
catch (KreuzbergException ex)
{
    Console.WriteLine($"Error Code: {ex.Code}");
    Console.WriteLine($"Error Message: {ex.Message}");
}
catch (Exception ex)
{
    Console.WriteLine($"Unexpected error: {ex.Message}");
}
```
