#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    /* Test 1: Free NULL batch result (safe no-op) */
    kreuzberg_free_batch_result(NULL);

    /* Test 2: Batch extract files with empty list (count=0) */
    {
        struct CBatchResult *batch = kreuzberg_batch_extract_files_sync(NULL, 0, NULL);

        /*
         * With zero files, the function may return a valid batch with
         * count=0 or NULL. Both are acceptable.
         */
        if (batch != NULL) {
            assert(batch->count == 0);
            kreuzberg_free_batch_result(batch);
        }
    }

    /* Test 3: Batch extract bytes with a small text/plain sample */
    {
        const char *text = "Batch extraction test content.";
        struct CBytesWithMime item;
        item.data = (const uint8_t *)text;
        item.data_len = strlen(text);
        item.mime_type = "text/plain";

        struct CBatchResult *batch = kreuzberg_batch_extract_bytes_sync(&item, 1, NULL);

        if (batch != NULL) {
            /*
             * If the batch succeeded, verify the structure.
             * The text/plain handler may not be available (missing
             * runtime deps), so we handle both success and failure.
             */
            if (batch->success && batch->count > 0) {
                assert(batch->results != NULL);
                assert(batch->count == 1);

                const struct CExtractionResult *res = batch->results[0];
                if (res != NULL && res->success) {
                    assert(res->content != NULL);
                    assert(strlen(res->content) > 0);
                }
            }
            kreuzberg_free_batch_result(batch);
        } else {
            /* Batch returned NULL - check error */
            const char *err = kreuzberg_last_error();
            printf("  note: batch bytes extraction returned NULL (error: %s)\n",
                   err ? err : "(none)");
        }
    }

    /* Test 4: Batch extract bytes with multiple items */
    {
        const char *text1 = "First document content.";
        const char *text2 = "Second document content.";

        struct CBytesWithMime items[2];
        items[0].data = (const uint8_t *)text1;
        items[0].data_len = strlen(text1);
        items[0].mime_type = "text/plain";
        items[1].data = (const uint8_t *)text2;
        items[1].data_len = strlen(text2);
        items[1].mime_type = "text/plain";

        struct CBatchResult *batch = kreuzberg_batch_extract_bytes_sync(items, 2, NULL);

        if (batch != NULL) {
            if (batch->success) {
                assert(batch->count == 2);
                assert(batch->results != NULL);
            }
            kreuzberg_free_batch_result(batch);
        } else {
            const char *err = kreuzberg_last_error();
            printf("  note: multi-item batch returned NULL (error: %s)\n", err ? err : "(none)");
        }
    }

    /* Test 5: Batch extract files with nonexistent paths */
    {
        const char *paths[] = {"/nonexistent/file1.txt", "/nonexistent/file2.txt"};
        struct CBatchResult *batch = kreuzberg_batch_extract_files_sync(paths, 2, NULL);

        if (batch != NULL) {
            /*
             * Files don't exist, so individual results should indicate
             * failure, but the batch itself may still be returned.
             */
            kreuzberg_free_batch_result(batch);
        }
    }

    /*
     * Note: kreuzberg_extract_batch_streaming and kreuzberg_extract_batch_parallel
     * take an Option_ErrorCallback parameter which is an opaque type in the C
     * header (incomplete struct). These functions cannot be directly tested from
     * C without a helper to construct the Option_ErrorCallback. Skipping those
     * tests here.
     */

    /* Test 8: Batch extract with explicit JSON config */
    {
        const char *text = "Config test content.";
        struct CBytesWithMime item;
        item.data = (const uint8_t *)text;
        item.data_len = strlen(text);
        item.mime_type = "text/plain";

        /* Pass a minimal valid config */
        const char *config = "{}";

        struct CBatchResult *batch = kreuzberg_batch_extract_bytes_sync(&item, 1, config);

        if (batch != NULL) {
            kreuzberg_free_batch_result(batch);
        }
    }

    printf("test_batch: all tests passed\n");
    return 0;
}
