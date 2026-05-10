```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"force_ocr\": true,"
        "\"ocr\": {\"backend\": \"tesseract\", \"language\": \"eng\"}"
        "}";

    KREUZBERGExtractionConfig *config =
        kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config init failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file("scanned.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    char *detected_languages = kreuzberg_extraction_result_detected_languages(result);
    printf("detected languages: %s\n",
           detected_languages ? detected_languages : "(none)");
    kreuzberg_free_string(detected_languages);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
