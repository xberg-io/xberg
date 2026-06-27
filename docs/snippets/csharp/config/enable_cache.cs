using Xberg;

var config = new ExtractionConfig
{
    UseCache = true
};

var result = XbergLib.Extract("document.pdf", config);
