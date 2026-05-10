```csharp title="C#"
using Kreuzberg;

var names = new List<string>
{
    "custom-json-extractor",
    "word_count",
    "cloud-ocr",
    "min_length_validator"
};

KreuzbergLib.UnregisterDocumentExtractor(names[0]);
KreuzbergLib.UnregisterPostProcessor(names[1]);
KreuzbergLib.UnregisterOcrBackend(names[2]);
KreuzbergLib.UnregisterValidator(names[3]);
```
