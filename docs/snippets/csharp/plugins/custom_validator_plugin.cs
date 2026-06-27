using Xberg;

class MinLengthValidator : IValidator
{
    private readonly int _minLength;

    public MinLengthValidator(int minLength)
    {
        _minLength = minLength;
    }

    public string Name => "min-length";
    public int Priority => 10;

    public void Validate(ExtractionResult result)
    {
        if (result.Content.Length < _minLength)
        {
            throw new XbergValidationException(
                $"Content too short: {result.Content.Length} < {_minLength}"
            );
        }
    }
}

class QualityScoreValidator : IValidator
{
    private readonly double _minScore;

    public QualityScoreValidator(double minScore)
    {
        _minScore = minScore;
    }

    public string Name => "quality-score";
    public int Priority => 5;

    public void Validate(ExtractionResult result)
    {
        var score = result.QualityScore;

        if (score < _minScore)
        {
            throw new XbergValidationException(
                $"Quality score too low: {score:F2} < {_minScore:F2}"
            );
        }
    }
}

class ContentValidValidator : IValidator
{
    public string Name => "content-valid";
    public int Priority => 20;

    public void Validate(ExtractionResult result)
    {
        if (string.IsNullOrWhiteSpace(result.Content))
        {
            throw new XbergValidationException("Extracted content is empty or whitespace");
        }

        if (result.Content.Length < 10)
        {
            throw new XbergValidationException("Extracted content is too short (minimum 10 characters)");
        }
    }
}

class Program
{
    static void Main()
    {
        var minLengthValidator = new MinLengthValidator(minLength: 50);
        var qualityValidator = new QualityScoreValidator(minScore: 0.7);
        var contentValidator = new ContentValidValidator();

        XbergLib.RegisterValidator(minLengthValidator);
        XbergLib.RegisterValidator(qualityValidator);
        XbergLib.RegisterValidator(contentValidator);

        try
        {
            var config = new ExtractionConfig
            {
                EnableQualityProcessing = true
            };

            var result = XbergLib.ExtractSync("document.pdf", config);

            Console.WriteLine("All validations passed");
            Console.WriteLine($"Content length: {result.Content.Length}");
        }
        catch (XbergValidationException ex)
        {
            Console.WriteLine($"Validation failed: {ex.Message}");
        }
        catch (XbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
