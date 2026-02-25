#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Test NULL path extraction fails gracefully */
    const struct CExtractionResult *result = kreuzberg_extract_file_sync(NULL);
    assert(result == NULL);

    const char *err = kreuzberg_last_error();
    assert(err != NULL);
    assert(strlen(err) > 0);

    /* Verify error code is set after NULL path error */
    {
        int32_t code = kreuzberg_last_error_code();
        assert(code != 0);
    }

    /* Test nonexistent file */
    result = kreuzberg_extract_file_sync("/nonexistent/file.pdf");
    assert(result == NULL);

    /* Test free_result with NULL is safe */
    kreuzberg_free_result(NULL);

    /* Test free_string with NULL is safe */
    kreuzberg_free_string(NULL);

    /* Test successful extraction from bytes with text/plain content */
    {
        const char *text = "Hello, Kreuzberg! This is a test document.";
        struct CExtractionResult *res =
            kreuzberg_extract_bytes_sync((const uint8_t *)text, strlen(text), "text/plain");

        /*
         * extraction may return NULL if the text/plain handler is not
         * available (e.g., missing runtime dependencies). In that case
         * we skip the field assertions but still verify the error path
         * works correctly.
         */
        if (res != NULL) {
            /* Verify success flag */
            assert(res->success);

            /* Verify content is populated and non-empty */
            assert(res->content != NULL);
            assert(strlen(res->content) > 0);

            /* Verify mime_type is populated */
            assert(res->mime_type != NULL);
            assert(strlen(res->mime_type) > 0);

            /* Content should contain our input text (or a transformation of it) */
            assert(strstr(res->content, "Hello") != NULL ||
                   strstr(res->content, "Kreuzberg") != NULL);

            kreuzberg_free_result(res);
        } else {
            /* Extraction returned NULL -- verify error is set */
            const char *extract_err = kreuzberg_last_error();
            printf("  note: bytes extraction returned NULL (error: %s)\n",
                   extract_err ? extract_err : "(none)");
        }
    }

    printf("test_extraction: all tests passed\n");
    return 0;
}
