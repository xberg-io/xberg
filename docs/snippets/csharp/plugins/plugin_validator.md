```csharp title="C#"
using Xberg;

var validator = new ContentTypeValidator("application/pdf", "text/plain");
ValidatorRegistry.Register(validator);

public class ContentTypeValidator : IValidator
{
    private readonly string[] _allowedMimeTypes;

    public ContentTypeValidator(params string[] allowedMimeTypes)
    {
        _allowedMimeTypes = allowedMimeTypes;
    }

    public string Name => "content-type-validator";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine($"Content type validator initialized with types: {string.Join(", ", _allowedMimeTypes)}");
    }

    public void Shutdown()
    {
        Console.WriteLine("Content type validator shut down");
    }

    public void Validate(ExtractedDocument result, ExtractionConfig config)
    {
        if (!_allowedMimeTypes.Contains(result.MimeType))
        {
            throw new XbergException(
                $"MIME type {result.MimeType} not allowed. Allowed types: {string.Join(", ", _allowedMimeTypes)}",
                1002
            );
        }
    }

    public bool ShouldValidate(ExtractedDocument result, ExtractionConfig config)
    {
        return true;
    }

    public int Priority()
    {
        return 50;
    }
}
```
