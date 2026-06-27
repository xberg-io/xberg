```c title="C"
#include "xberg.h"
#include <stdio.h>

int main(void) {
    /* Cloud OCR backends are registered as custom plugins via the Rust core. */
    /* Select a registered cloud backend by name through the OCR config. */
    const char *config_json =
        "{"
        "\"ocr\": {"
        "\"backend\": \"cloud-ocr\","
        "\"language\": \"eng\""
        "}"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract_sync("scanned.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *content = xberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    xberg_free_string(content);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
