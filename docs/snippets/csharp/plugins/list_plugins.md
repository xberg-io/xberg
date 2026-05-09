```csharp title="C#"
using Kreuzberg;

var extractors = KreuzbergLib.ListDocumentExtractors();
Console.WriteLine("Registered extractors: " + string.Join(", ", extractors));

var ocrBackends = KreuzbergLib.ListOcrBackends();
Console.WriteLine("Registered OCR backends: " + string.Join(", ", ocrBackends));

var processors = KreuzbergLib.ListPostProcessors();
Console.WriteLine("Registered post-processors: " + string.Join(", ", processors));

var validators = KreuzbergLib.ListValidators();
Console.WriteLine("Registered validators: " + string.Join(", ", validators));
```
