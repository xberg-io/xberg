```c title="C"
#include <xberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"force_ocr\": true,"
        "\"ocr\": {\"backend\": \"tesseract\", \"language\": \"eng\"}"
        "}";

    XBERGExtractionConfig *config =
        xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config init failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract("scanned.pdf", NULL, config);
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

    char *detected_languages = xberg_extraction_result_detected_languages(result);
    printf("detected languages: %s\n",
           detected_languages ? detected_languages : "(none)");
    xberg_free_string(detected_languages);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
