```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    ResultFormat = ResultFormat.ElementBased
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);

if (result.Elements != null)
{
    foreach (var element in result.Elements)
    {
        Console.WriteLine($"Type: {element.ElementType}");
        Console.WriteLine($"Text: {element.Text.Substring(0, Math.Min(100, element.Text.Length))}");

        if (element.Metadata.PageNumber.HasValue)
        {
            Console.WriteLine($"Page: {element.Metadata.PageNumber}");
        }

        if (element.Metadata.Coordinates != null)
        {
            Console.WriteLine($"Coords: ({element.Metadata.Coordinates.X0}, {element.Metadata.Coordinates.Y0})");
        }

        Console.WriteLine("---");
    }

    var titles = result.Elements
        .Where(e => e.ElementType == ElementType.Title)
        .ToList();

    Console.WriteLine($"Found {titles.Count} titles");
}
```
