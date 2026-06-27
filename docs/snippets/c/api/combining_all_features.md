```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* Combine chunking, OCR, image extraction, and Markdown output in one config. */
    const char *config_json =
        "{"
        "\"output_format\": \"markdown\","
        "\"force_ocr\": true,"
        "\"ocr\": {\"backend\": \"tesseract\", \"languages\": [\"eng\", \"deu\"]},"
        "\"chunking\": {\"chunker_type\": \"character\", \"max_characters\": 1024, \"overlap\": 128, \"trim\": true},"
        "\"images\": {\"extract_images\": true, \"target_dpi\": 300, \"inject_placeholders\": true}"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract("document.pdf", NULL, config);
    if (!result) {
        int32_t code = xberg_last_error_code();
        const char *message = xberg_last_error_context();
        fprintf(stderr, "extraction failed (code %d): %s\n",
                code, message ? message : "(no message)");
        xberg_extraction_config_free(config);
        return code != 0 ? code : 1;
    }

    char *content = xberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    xberg_free_string(content);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
