```c title="C"
#include <kreuzberg.h>
#include <stdio.h>

static void print_plugin_list(const char *label, char *json) {
    if (!json) {
        fprintf(stderr, "list %s failed (code %d): %s\n",
                label,
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return;
    }
    printf("%s: %s\n", label, json);
    kreuzberg_free_string(json);
}

int main(void) {
    print_plugin_list("document extractors", kreuzberg_list_document_extractors());
    print_plugin_list("OCR backends", kreuzberg_list_ocr_backends());
    print_plugin_list("post-processors", kreuzberg_list_post_processors());
    print_plugin_list("validators", kreuzberg_list_validators());
    print_plugin_list("embedding presets", kreuzberg_list_embedding_presets());
    return 0;
}
```
