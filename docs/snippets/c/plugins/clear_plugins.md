```c title="C"
#include <kreuzberg.h>
#include <stdio.h>

int main(void) {
    if (kreuzberg_clear_post_processors() != 0) {
        fprintf(stderr, "clear post-processors failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    if (kreuzberg_clear_ocr_backends() != 0) {
        fprintf(stderr, "clear OCR backends failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    if (kreuzberg_clear_validators() != 0) {
        fprintf(stderr, "clear validators failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    printf("All plugins cleared\n");
    return 0;
}
```
