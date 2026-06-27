```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"keywords\": {"
        "\"algorithm\": \"yake\","
        "\"max_keywords\": 10,"
        "\"min_score\": 0.3,"
        "\"ngram_range\": [1, 3],"
        "\"language\": \"en\""
        "}"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractInput *input = xberg_extract_input_from_uri("document.pdf");
    if (!input) {
        fprintf(stderr, "input create failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    XBERGExtractionResult *output = xberg_extract(input, config);
    if (!output) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extract_input_free(input);
        xberg_extraction_config_free(config);
        return 1;
    }

    char *results_json = xberg_extraction_result_results(output);
    printf("results[0].extracted_keywords: %s\n", results_json ? results_json : "[]");
    xberg_free_string(results_json);

    xberg_extraction_result_free(output);
    xberg_extract_input_free(input);
    xberg_extraction_config_free(config);
    return 0;
}
```
