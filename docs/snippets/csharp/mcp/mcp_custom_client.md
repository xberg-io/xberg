<!-- snippet:syntax-only -->

```csharp title="C#"
using System.Diagnostics;
using System.Text.Json;

var processInfo = new ProcessStartInfo
{
    FileName = "xberg",
    Arguments = "mcp",
    UseShellExecute = false,
    RedirectStandardInput = true,
    RedirectStandardOutput = true,
    RedirectStandardError = true,
};

using var process = Process.Start(processInfo)
    ?? throw new InvalidOperationException("Failed to start xberg mcp");

var request = new
{
    method = "tools/call",
    @params = new
    {
        name = "extract",
        arguments = new { path = "document.pdf" },
    },
};

await process.StandardInput.WriteLineAsync(JsonSerializer.Serialize(request));
await process.StandardInput.FlushAsync();

var line = await process.StandardOutput.ReadLineAsync();
Console.WriteLine(line);

process.StandardInput.Close();
await process.WaitForExitAsync();
```
