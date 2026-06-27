using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true,
        EnabledProcessors = new List<string> { "normalize_whitespace", "remove_diacritics" }
    }
};

var result = XbergLib.ExtractSync("document.pdf", config);
