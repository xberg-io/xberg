```csharp title="C#"
using Xberg;

var validator = new MinimumLengthValidator();
ValidatorRegistry.Register(validator);

public class MinimumLengthValidator : IValidator
{
    private const int MinimumLength = 10;

    public string Name => "min-length-validator";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine($"Minimum length validator initialized (min: {MinimumLength})");
    }

    public void Shutdown()
    {
        Console.WriteLine("Minimum length validator shut down");
    }

    public void Validate(ExtractedDocument result, ExtractionConfig config)
    {
        if (result.Content.Length < MinimumLength)
        {
            throw new XbergException(
                $"Content length {result.Content.Length} is below minimum {MinimumLength}",
                1001
            );
        }
    }

    public bool ShouldValidate(ExtractedDocument result, ExtractionConfig config)
    {
        return !string.IsNullOrEmpty(result.Content);
    }

    public int Priority()
    {
        return 50;
    }
}
```
