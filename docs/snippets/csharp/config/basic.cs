using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = XbergLib.Extract("document.pdf", config);
