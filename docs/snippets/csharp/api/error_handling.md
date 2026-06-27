```csharp title="C#"
using Xberg;

try
{
    var result = XbergLib.ExtractSync("nonexistent.pdf", null, null);
    Console.WriteLine(result.Content);
}
catch (XbergException ex)
{
    Console.WriteLine($"Error Code: {ex.Code}");
    Console.WriteLine($"Error Message: {ex.Message}");
}
catch (Exception ex)
{
    Console.WriteLine($"Unexpected error: {ex.Message}");
}
```
