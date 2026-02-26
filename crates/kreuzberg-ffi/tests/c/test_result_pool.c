#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Test 1: Create pool with capacity 10, verify non-NULL */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(10);
        assert(pool != NULL);

        /* Test 2: Check initial stats (should be empty) */
        struct CResultPoolStats stats = kreuzberg_result_pool_stats(pool);
        assert(stats.current_count == 0);
        assert(stats.capacity == 10);
        assert(stats.total_allocations == 0);
        assert(stats.growth_events == 0);
        assert(stats.estimated_memory_bytes == 0);

        /* Test 3: Reset pool (should not crash) */
        kreuzberg_result_pool_reset(pool);

        /* Verify stats are still empty after reset */
        stats = kreuzberg_result_pool_stats(pool);
        assert(stats.current_count == 0);

        /* Test 4: Free pool */
        kreuzberg_result_pool_free(pool);
    }

    /* Test 5: Free NULL pool (should be safe no-op) */
    kreuzberg_result_pool_free(NULL);

    /* Test 6: Create pool with capacity 0 */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(0);
        /* Pool may still be created (just with zero initial capacity) */
        if (pool != NULL) {
            struct CResultPoolStats stats = kreuzberg_result_pool_stats(pool);
            assert(stats.current_count == 0);
            assert(stats.capacity == 0);
            kreuzberg_result_pool_free(pool);
        }
    }

    /* Test 7: Create pool with large capacity */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(1000);
        assert(pool != NULL);

        struct CResultPoolStats stats = kreuzberg_result_pool_stats(pool);
        assert(stats.capacity == 1000);
        assert(stats.current_count == 0);
        assert(stats.total_allocations == 0);

        kreuzberg_result_pool_free(pool);
    }

    /* Test 8: Multiple reset cycles should not leak or crash */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(5);
        assert(pool != NULL);

        kreuzberg_result_pool_reset(pool);
        kreuzberg_result_pool_reset(pool);
        kreuzberg_result_pool_reset(pool);

        struct CResultPoolStats stats = kreuzberg_result_pool_stats(pool);
        assert(stats.current_count == 0);

        kreuzberg_result_pool_free(pool);
    }

    /*
     * Test 9: Attempt extraction into pool with a nonexistent file.
     * This exercises the pool extraction path even if the file is missing.
     */
    {
        struct ResultPool *pool = kreuzberg_result_pool_new(10);
        assert(pool != NULL);

        const struct CExtractionResultView *view =
            kreuzberg_extract_file_into_pool("/nonexistent/file.txt", NULL, pool);

        /* Extraction of nonexistent file should return NULL */
        assert(view == NULL);

        /* Pool stats should still show 0 results */
        struct CResultPoolStats stats = kreuzberg_result_pool_stats(pool);
        assert(stats.current_count == 0);

        kreuzberg_result_pool_free(pool);
    }

    printf("test_result_pool: all tests passed\n");
    return 0;
}
