```c title="C"
#include <xberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    XBERGExtractionResult *result =
        xberg_extract("document.pdf", NULL, NULL);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    char *content = xberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    xberg_free_string(content);

    xberg_extraction_result_free(result);
    return 0;
}
```
