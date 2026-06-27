using Xberg;

var config = new ExtractionConfig
{
    UseCache = false
};

var result = XbergLib.ExtractSync("document.pdf", config);
