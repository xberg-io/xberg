```csharp title="Element-Based Output (C#)"
using Xberg;

// Configure element-based output
var config = new ExtractionConfig
{
    OutputFormat = OutputFormat.ElementBased
};

// Extract document
var result = Xberg.ExtractSync("document.pdf", config);

// Access elements
foreach (var element in result.Elements)
{
    Console.WriteLine($"Type: {element.ElementType}");

    var text = element.Text.Length > 100
        ? element.Text.Substring(0, 100)
        : element.Text;
    Console.WriteLine($"Text: {text}");

    if (element.Metadata.PageNumber.HasValue)
    {
        Console.WriteLine($"Page: {element.Metadata.PageNumber}");
    }

    if (element.Metadata.Coordinates != null)
    {
        var coords = element.Metadata.Coordinates;
        Console.WriteLine($"Coords: ({coords.Left}, {coords.Top}) - ({coords.Right}, {coords.Bottom})");
    }

    Console.WriteLine("---");
}

// Filter by element type
var titles = result.Elements
    .Where(e => e.ElementType == "title");

foreach (var title in titles)
{
    var level = title.Metadata.Additional.TryGetValue("level", out var levelValue)
        ? levelValue.ToString()
        : "unknown";
    Console.WriteLine($"[{level}] {title.Text}");
}
```
