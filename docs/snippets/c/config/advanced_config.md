```c title="C"
#include "xberg.h"
#include <stdio.h>

int main(void) {
    struct ConfigBuilder *builder = xberg_config_builder_new();
    xberg_config_builder_set_use_cache(builder, 1);
    xberg_config_builder_set_include_document_structure(builder, 1);
    xberg_config_builder_set_ocr(builder,
        "{\"tesseract\":{\"language\":\"eng\"}}");

    ExtractionConfig *config = xberg_config_builder_build(builder);

    struct CExtractionResult *result =
        xberg_extract_sync_with_config("scan.pdf",
            xberg_config_to_json(config));
    if (result && result->success) {
        printf("%s\n", result->content);
    }

    xberg_free_result(result);
    xberg_config_free(config);
    return 0;
}
```
