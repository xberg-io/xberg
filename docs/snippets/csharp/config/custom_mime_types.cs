using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = XbergLib.Extract(
    new BytesWithMime(fileBytes, "application/pdf"),
    config
);

var mimeType = result.MimeType;
