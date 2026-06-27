using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = XbergLib.ExtractSync("document.pdf", config);

if (!result.Success)
{
    if (result.Metadata?.Error != null)
    {
        var errorType = result.Metadata.Error.ErrorType;
        var errorMessage = result.Metadata.Error.Message;
    }
}
