```csharp title="C#"
using Kreuzberg;

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

    public void Validate(ExtractionResult result, ExtractionConfig config)
    {
        if (!_allowedMimeTypes.Contains(result.MimeType))
        {
            throw new KreuzbergException(
                $"MIME type {result.MimeType} not allowed. Allowed types: {string.Join(", ", _allowedMimeTypes)}",
                1002
            );
        }
    }

    public bool ShouldValidate(ExtractionResult result, ExtractionConfig config)
    {
        return true;
    }

    public int Priority()
    {
        return 50;
    }
}

var validator = new ContentTypeValidator("application/pdf", "text/plain");
ValidatorRegistry.Register(validator);
```
