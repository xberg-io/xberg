<!-- snippet:syntax-only -->

```csharp title="C#"
using System.Diagnostics;

var processInfo = new ProcessStartInfo
{
    FileName = "kreuzberg",
    Arguments = "mcp",
    UseShellExecute = false,
    RedirectStandardInput = true,
    RedirectStandardOutput = true,
    RedirectStandardError = true,
};

using var server = Process.Start(processInfo)
    ?? throw new InvalidOperationException("Failed to start kreuzberg mcp");

await server.WaitForExitAsync();
```
