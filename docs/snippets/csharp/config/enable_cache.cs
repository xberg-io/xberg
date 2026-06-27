using Xberg;

var config = new ExtractionConfig
{
    UseCache = true
};

var result = XbergLib.ExtractSync("document.pdf", config);
