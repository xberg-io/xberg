using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
