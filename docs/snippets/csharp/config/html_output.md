```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    OutputFormat = OutputFormat.Html,
    HtmlOutput = new HtmlOutputConfig
    {
        Theme = HtmlTheme.GitHub,
        EmbedCss = true,
        ClassPrefix = "kb-"
    }
};

var result = await KreuzbergLib.ExtractFile("document.pdf", null, config);
Console.WriteLine(result.Content);
```
