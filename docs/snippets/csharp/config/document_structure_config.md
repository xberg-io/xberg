```csharp title="Document Structure Config (C#)"
using Kreuzberg;

var config = new ExtractionConfig
{
    IncludeDocumentStructure = true
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);

if (result.Document is not null)
{
    foreach (var node in result.Document.Nodes)
    {
        Console.WriteLine($"[{node.Content.NodeType}]");
    }
}
```
