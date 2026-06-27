```csharp title="basic_cli.cs"
using System;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.Threading.Tasks;
using Xberg;

var rootCommand = new RootCommand("Xberg document extraction CLI");

var extractCommand = new Command("extract-file", "Extract text from a document file");
var filePath = new Argument<string>("path", "Path to the document file");
var outputFormat = new Option<string>(
    new[] { "-f", "--format" },
    getDefaultValue: () => "text",
    "Output format (text, json)"
);

extractCommand.AddArgument(filePath);
extractCommand.AddOption(outputFormat);

extractCommand.SetHandler(async (path, format) =>
{
    try
    {
        var result = await XbergLib.ExtractAsync(path);

        if (format == "json")
        {
            Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(result));
        }
        else
        {
            Console.WriteLine(result.Content);
        }
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"Error: {ex.Message}");
        Environment.Exit(1);
    }
}, filePath, outputFormat);

rootCommand.AddCommand(extractCommand);

return await rootCommand.InvokeAsync(args);
```
