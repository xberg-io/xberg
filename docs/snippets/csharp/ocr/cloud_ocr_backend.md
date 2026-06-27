```csharp title="C#"
using Xberg;
using System.Collections.Generic;

public class CloudOcrBackend : IOcrBackend
{
    private string _apiKey;

    public string Name => "cloud-ocr";
    public string Version => "1.0.0";

    public CloudOcrBackend(string apiKey)
    {
        _apiKey = apiKey;
    }

    public void Initialize()
    {
    }

    public void Shutdown()
    {
    }

    public ExtractedDocument ProcessImage(byte[] imageBytes, OcrConfig config)
    {
        // Call cloud OCR API with imageBytes and config.Language
        // Return ExtractedDocument with extracted text
        throw new NotImplementedException();
    }

    public ExtractedDocument ProcessImageFile(string path, OcrConfig config)
    {
        var imageBytes = File.ReadAllBytes(path);
        return ProcessImage(imageBytes, config);
    }

    public bool SupportsLanguage(string language)
    {
        return SupportedLanguages().Contains(language);
    }

    public OcrBackendType BackendType()
    {
        return OcrBackendType.Cloud;
    }

    public List<string> SupportedLanguages()
    {
        return new List<string> { "eng", "deu", "fra" };
    }

    public bool SupportsTableDetection()
    {
        return false;
    }

    public bool SupportsDocumentProcessing()
    {
        return false;
    }

    public ExtractedDocument ProcessDocument(string path, OcrConfig config)
    {
        throw new NotSupportedException("Document processing not supported by CloudOcrBackend");
    }
}

class Program
{
    static void Main()
    {
        // Register the backend
        var backend = new CloudOcrBackend(apiKey: "your-api-key");
        OcrBackendBridge.Register(backend);
    }
}
```
