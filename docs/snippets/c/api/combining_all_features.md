```c title="C"
#include "kreuzberg.h"
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

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file("document.pdf", NULL, config);
    if (!result) {
        int32_t code = kreuzberg_last_error_code();
        const char *message = kreuzberg_last_error_context();
        fprintf(stderr, "extraction failed (code %d): %s\n",
                code, message ? message : "(no message)");
        kreuzberg_extraction_config_free(config);
        return code != 0 ? code : 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
