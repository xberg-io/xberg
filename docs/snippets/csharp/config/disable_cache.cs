using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = false
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
