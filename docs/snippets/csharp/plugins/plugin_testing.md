```csharp title="C#"
using Xberg;
using Xunit;

public class CustomValidatorTests
{
    [Fact]
    public void TestValidatorRegistration()
    {
        var validator = new TestValidator();
        ValidatorRegistry.Register(validator);

        var validators = XbergLib.ListValidators();
        Assert.Contains("test-validator", validators);
    }

    [Fact]
    public void TestValidatorProcessing()
    {
        var result = new ExtractedDocument
        {
            Content = "Test content with some length",
            MimeType = "text/plain"
        };

        var config = new ExtractionConfig();
        var validator = new TestValidator();

        validator.Initialize();
        Assert.True(validator.ShouldValidate(result, config));
        validator.Validate(result, config);
        validator.Shutdown();
    }
}

public class TestValidator : IValidator
{
    public string Name => "test-validator";
    public string Version => "1.0.0";

    public void Initialize() { }
    public void Shutdown() { }

    public void Validate(ExtractedDocument result, ExtractionConfig config)
    {
        if (string.IsNullOrEmpty(result.Content))
        {
            throw new XbergException("Content cannot be empty", 1000);
        }
    }

    public bool ShouldValidate(ExtractedDocument result, ExtractionConfig config)
    {
        return !string.IsNullOrEmpty(result.Content);
    }

    public int Priority() => 50;
}
```
