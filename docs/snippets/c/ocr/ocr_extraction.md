```c title="C"
#include "xberg.h"
#include <stdio.h>

int main(void) {
    struct ConfigBuilder *builder = xberg_config_builder_new();
    xberg_config_builder_set_ocr(builder,
        "{\"tesseract\":{\"language\":\"eng\"}}");
    ExtractionConfig *config = xberg_config_builder_build(builder);

    char *config_json = xberg_config_to_json(config);
    struct CExtractionResult *result =
        xberg_extract_sync_with_config("scanned.png", config_json);

    if (result && result->success) {
        printf("OCR text: %s\n", result->content);
    } else {
        fprintf(stderr, "OCR error: %s\n", xberg_get_error_details().message);
    }

    xberg_free_result(result);
    xberg_free_string(config_json);
    xberg_config_free(config);
    return 0;
}
```
