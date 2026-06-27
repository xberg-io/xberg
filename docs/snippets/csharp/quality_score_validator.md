```csharp title="C#"
using Xberg;

public class QualityValidator : IValidator
{
    public string Name() => "quality-validator";
    public string Version() => "1.0.0";

    public void Validate(ExtractedDocument result)
    {
        var score = result.QualityScore;

        if (score < 0.5)
            throw new ValidationError($"Quality score too low: {score:F2}");
    }

    public bool ShouldValidate(Dictionary<string, object> result) => true;
    public int Priority() => 100;
    public void Initialize() { }
    public void Shutdown() { }
}

class Program
{
    static void Main()
    {
        var validator = new QualityValidator();
        XbergLib.RegisterValidator(validator);
    }
}
```
