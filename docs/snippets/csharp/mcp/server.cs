```csharp title="server.cs"
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;

class McpServer
{
    public static async Task Main(string[] args)
    {
        var processInfo = new ProcessStartInfo
        {
            FileName = "xberg",
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

class McpServerProgram
{
    public static async Task Main()
    {
        var server = new XbergMcpServer();

        server.RegisterTool("extract", new Dictionary<string, object>
        {
            { "description", "Extract text from a document file" },
            { "parameters", new { path = "string" } }
        });

        server.RegisterTool("extract", new Dictionary<string, object>
        {
            { "description", "Extract text from document bytes" },
            { "parameters", new { data = "string", mimeType = "string" } }
        });

        await server.StartAsync();
    }
}
```
