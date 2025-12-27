```csharp
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;

class McpServer
{
    public static async Task Main(string[] args)
    {
        var processInfo = new ProcessStartInfo
        {
            FileName = "kreuzberg",
            Arguments = "mcp",
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };

        var process = Process.Start(processInfo);

        await Task.Delay(Timeout.Infinite);
    }
}

using System.Net.Http;
using System.Text.Json;

class McpServerProgram
{
    public static async Task Main()
    {
        var server = new KreuzbergMcpServer();

        server.RegisterTool("extract_file", new Dictionary<string, object>
        {
            { "description", "Extract text from a document file" },
            { "parameters", new { path = "string" } }
        });

        server.RegisterTool("extract_bytes", new Dictionary<string, object>
        {
            { "description", "Extract text from document bytes" },
            { "parameters", new { data = "string", mimeType = "string" } }
        });

        await server.StartAsync();
    }
}
```
