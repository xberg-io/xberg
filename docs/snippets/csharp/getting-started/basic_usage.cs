using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = XbergClient.ExtractSync("document.pdf", config);

Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
