```csharp title="C#"
using Kreuzberg;

var extractors = KreuzbergLib.ListDocumentExtractors();
var processors = KreuzbergLib.ListPostProcessors();
var ocrBackends = KreuzbergLib.ListOcrBackends();
var validators = KreuzbergLib.ListValidators();

Console.WriteLine($"Extractors: {string.Join(", ", extractors)}");
Console.WriteLine($"Processors: {string.Join(", ", processors)}");
Console.WriteLine($"OCR backends: {string.Join(", ", ocrBackends)}");
Console.WriteLine($"Validators: {string.Join(", ", validators)}");
```
