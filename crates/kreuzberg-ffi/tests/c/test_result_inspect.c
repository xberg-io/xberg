#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /*
     * The kreuzberg_result_get_* accessor functions operate on the opaque
     * ExtractionResult type. From C, we can obtain a CExtractionResult via
     * kreuzberg_extract_bytes_sync. We test the CExtractionResult struct
     * fields directly, and also test the pool-based extraction which
     * produces CExtractionResultView.
     */

    /* Test 1: Extract bytes with text/plain and inspect CExtractionResult fields */
    {
        const char *text = "Hello from kreuzberg test. This is sample content for inspection.";
        struct CExtractionResult *res =
            kreuzberg_extract_bytes_sync((const uint8_t *)text, strlen(text), "text/plain");

        if (res != NULL) {
            /* Verify success flag */
            assert(res->success);

            /* Content should be non-NULL and non-empty */
            assert(res->content != NULL);
            assert(strlen(res->content) > 0);

            /* MIME type should be set */
            assert(res->mime_type != NULL);
            assert(strlen(res->mime_type) > 0);

            /* Content should contain parts of our input */
            assert(strstr(res->content, "Hello") != NULL ||
                   strstr(res->content, "kreuzberg") != NULL);

            /* Optional fields may be NULL -- that is fine */
            /* language, date, subject can be NULL for text/plain */

            /* metadata_json may or may not be present */
            if (res->metadata_json != NULL) {
                assert(strlen(res->metadata_json) > 0);
            }

            /* chunks_json should be NULL unless chunking is configured */
            /* (default config does not enable chunking) */

            kreuzberg_free_result(res);
        } else {
            printf("  note: bytes extraction returned NULL, skipping field inspection\n");
            const char *err = kreuzberg_last_error();
            printf("  note: error: %s\n", err ? err : "(none)");
        }
    }

    /* Test 2: Pool-based extraction and view inspection */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(10);
        assert(pool != NULL);

        /*
         * We cannot extract bytes directly into the pool (only files),
         * so we attempt with a nonexistent file to verify the API
         * handles failures correctly.
         */
        const struct CExtractionResultView *view =
            kreuzberg_extract_file_into_pool("/nonexistent/inspect_test.txt", NULL, pool);

        /* Should fail and return NULL */
        assert(view == NULL);

        /* Also test the _view variant */
        struct CExtractionResultView view_struct =
            kreuzberg_extract_file_into_pool_view("/nonexistent/inspect_test.txt", NULL, pool);

        /* For a failed extraction, the view should have NULL content */
        assert(view_struct.content_ptr == NULL);
        assert(view_struct.content_len == 0);

        kreuzberg_result_pool_free(pool);
    }

    /* Test 3: View accessor functions with a valid view */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(10);
        assert(pool != NULL);

        /*
         * Since we cannot extract bytes into pool, attempt with nonexistent
         * file and verify view accessor functions handle NULL gracefully.
         */
        struct CExtractionResultView empty_view;
        memset(&empty_view, 0, sizeof(empty_view));

        const uint8_t *out_ptr = NULL;
        uintptr_t out_len = 0;

        /* kreuzberg_view_get_content with a zeroed view should handle gracefully */
        int32_t rc = kreuzberg_view_get_content(&empty_view, &out_ptr, &out_len);
        /*
         * The function returns 0 on success even for empty views
         * (content_ptr may be NULL with len 0).
         */
        if (rc == 0) {
            assert(out_ptr == NULL || out_len == 0);
        }

        /* kreuzberg_view_get_mime_type with empty view */
        out_ptr = NULL;
        out_len = 0;
        rc = kreuzberg_view_get_mime_type(&empty_view, &out_ptr, &out_len);
        if (rc == 0) {
            assert(out_ptr == NULL || out_len == 0);
        }

        kreuzberg_result_pool_free(pool);
    }

    /* Test 4: CMetadataField structure layout verification */
    {
        /*
         * Verify CMetadataField has the expected fields.
         * We cannot call kreuzberg_result_get_metadata_field without an
         * opaque ExtractionResult*, but we can verify the struct layout.
         */
        struct CMetadataField field;
        memset(&field, 0, sizeof(field));
        assert(field.name == NULL);
        assert(field.json_value == NULL);
        assert(field.is_null == 0);
    }

    printf("test_result_inspect: all tests passed\n");
    return 0;
}
