```csharp title="C#"
using Xberg;

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

var result = await XbergLib.Extract("document.pdf", null, config);
Console.WriteLine(result.Content);
```
