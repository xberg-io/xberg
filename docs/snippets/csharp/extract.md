```csharp title="C#"
using Xberg;

var output = await XbergConverter.ExtractAsync(
    ExtractInput.Uri("document.pdf"),
    ExtractionConfig.Default()
);

Console.WriteLine(output.Results[0].Content);
```
