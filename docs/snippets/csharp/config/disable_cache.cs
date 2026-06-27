using Xberg;

var config = new ExtractionConfig
{
    UseCache = false
};

var result = XbergLib.Extract("document.pdf", config);
