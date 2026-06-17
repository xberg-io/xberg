/**
 * Kreuzberg C extraction wrapper for benchmark harness.
 *
 * Supports three modes:
 * - server: persistent process reading paths from stdin (default benchmark mode)
 * - sync: extract single file and print JSON to stdout
 * - batch: extract multiple files and print JSON array to stdout
 *
 * Build:
 *   cc -O2 -o kreuzberg_extract_c kreuzberg_extract_c.c \
 *     -I../../crates/kreuzberg-ffi -L../../target/release \
 *     -lkreuzberg_ffi -lpthread -ldl -lm
 *
 * Usage:
 *   ./kreuzberg_extract_c [--ocr|--no-ocr] <mode> [file_paths...]
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#ifdef __APPLE__
#include <mach/mach.h>
#else
#include <sys/resource.h>
#endif

#include "kreuzberg.h"

static bool debug_enabled = false;

#define DEBUG_LOG(fmt, ...)                                                                        \
    do {                                                                                           \
        if (debug_enabled)                                                                         \
            fprintf(stderr, "[DEBUG] " fmt "\n", ##__VA_ARGS__);                                   \
    } while (0)

/* -------------------------------------------------------------------------- */
/* Helpers                                                                     */
/* -------------------------------------------------------------------------- */

static double time_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec / 1e6;
}

static uint64_t peak_memory_bytes(void) {
#ifdef __APPLE__
    struct mach_task_basic_info info = {0};
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    if (task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&info, &count) ==
        KERN_SUCCESS) {
        return (uint64_t)info.resident_size_max;
    }
    return 0;
#else
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) == 0) {
        return (uint64_t)usage.ru_maxrss * 1024; /* Linux reports in KB */
    }
    return 0;
#endif
}

/**
 * Write a JSON-escaped version of `src` into `dst`.
 * Returns the number of bytes written (excluding NUL).
 * If dst is NULL, just counts the required length.
 */
static size_t json_escape(char *dst, const char *src, size_t src_len) {
    size_t written = 0;
    for (size_t i = 0; i < src_len; i++) {
        unsigned char c = (unsigned char)src[i];
        const char *esc = NULL;
        switch (c) {
        case '"':
            esc = "\\\"";
            break;
        case '\\':
            esc = "\\\\";
            break;
        case '\b':
            esc = "\\b";
            break;
        case '\f':
            esc = "\\f";
            break;
        case '\n':
            esc = "\\n";
            break;
        case '\r':
            esc = "\\r";
            break;
        case '\t':
            esc = "\\t";
            break;
        default:
            if (c < 0x20) {
                if (dst)
                    written += (size_t)sprintf(dst + written, "\\u%04x", c);
                else
                    written += 6;
                continue;
            }
            if (dst)
                dst[written] = (char)c;
            written++;
            continue;
        }
        size_t elen = strlen(esc);
        if (dst)
            memcpy(dst + written, esc, elen);
        written += elen;
    }
    if (dst)
        dst[written] = '\0';
    return written;
}

/**
 * Allocate and return a JSON-escaped copy of `src`.
 * Caller must free the result.
 */
static char *json_escape_alloc(const char *src) {
    if (!src)
        return NULL;
    size_t src_len = strlen(src);
    size_t needed = json_escape(NULL, src, src_len);
    char *buf = malloc(needed + 1);
    if (!buf)
        return NULL;
    json_escape(buf, src, src_len);
    return buf;
}

/* -------------------------------------------------------------------------- */
/* OCR detection (mirrors Go/Rust adapter logic)                              */
/* -------------------------------------------------------------------------- */

static bool determine_ocr_used(const char *mime_type, bool ocr_enabled) {
    if (!mime_type)
        return false;
    /* If extraction detected OCR format */
    if (strstr(mime_type, "image/") != NULL && ocr_enabled)
        return true;
    return false;
}

/* -------------------------------------------------------------------------- */
/* JSON request parsing (minimal — just extract "path" and "force_ocr")       */
/* -------------------------------------------------------------------------- */

/**
 * Parse a request line which is either a plain file path or a JSON object
 * like {"path": "/some/file.pdf", "force_ocr": true}.
 *
 * Sets *out_path to the path (caller must NOT free — points into `line`)
 * and *out_force_ocr to the force_ocr value.
 *
 * For JSON parsing, we extract values manually to avoid dependencies.
 */
static void parse_request(char *line, const char **out_path, bool *out_force_ocr) {
    /* Trim trailing whitespace/newline */
    size_t len = strlen(line);
    while (len > 0 && (line[len - 1] == '\n' || line[len - 1] == '\r' || line[len - 1] == ' ' ||
                       line[len - 1] == '\t')) {
        line[--len] = '\0';
    }

    *out_force_ocr = false;

    /* Check if it's JSON */
    if (len > 0 && line[0] == '{') {
        /* Find "path" field */
        const char *path_key = "\"path\"";
        char *p = strstr(line, path_key);
        if (p) {
            p += strlen(path_key);
            /* Skip colon and whitespace */
            while (*p == ' ' || *p == ':' || *p == '\t')
                p++;
            if (*p == '"') {
                p++;
                char *end = strchr(p, '"');
                if (end) {
                    *end = '\0';
                    *out_path = p;
                }
            }
        }
        /* Check force_ocr */
        if (strstr(line, "\"force_ocr\":true") || strstr(line, "\"force_ocr\": true")) {
            *out_force_ocr = true;
        }
    } else {
        *out_path = line;
    }
}

/* -------------------------------------------------------------------------- */
/* Extraction + JSON output                                                    */
/* -------------------------------------------------------------------------- */

/**
 * Build the config JSON string for OCR-enabled extraction.
 * Returns a static string pointer (no allocation needed).
 */
static const char *ocr_config_json(void) {
    return "{\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}";
}

/**
 * Print extraction result as JSON to stdout.
 * Does NOT print a trailing newline (caller decides).
 */
static void print_result_json(const struct CExtractionResult *result, double elapsed_ms,
                              bool ocr_used) {
    uint64_t mem = peak_memory_bytes();

    /* Escape content for JSON */
    char *escaped_content = result->content ? json_escape_alloc(result->content) : NULL;
    const char *content_str = escaped_content ? escaped_content : "";

    /* metadata_json is already a valid JSON object string from the FFI */
    const char *metadata_str = result->metadata_json ? result->metadata_json : "{}";

    printf("{\"content\":\"%s\",\"metadata\":%s,"
           "\"_extraction_time_ms\":%.2f,"
           "\"_ocr_used\":%s,"
           "\"_peak_memory_bytes\":%llu}",
           content_str, metadata_str, elapsed_ms, ocr_used ? "true" : "false",
           (unsigned long long)mem);

    free(escaped_content);
}

/**
 * Print an error result as JSON to stdout (with newline).
 */
static void print_error_json(const char *error_msg) {
    char *escaped = error_msg ? json_escape_alloc(error_msg) : NULL;
    const char *msg = escaped ? escaped : "unknown error";
    printf("{\"error\":\"%s\",\"_extraction_time_ms\":0,\"_ocr_used\":false}\n", msg);
    fflush(stdout);
    free(escaped);
}

/**
 * Extract a single file and return the result.
 * Returns NULL on error (prints error JSON if in server mode).
 */
static struct CExtractionResult *extract_file(const char *path, bool ocr_enabled, bool force_ocr) {
    struct CExtractionResult *result;

    if (ocr_enabled || force_ocr) {
        result = kreuzberg_extract_file_sync_with_config(path, ocr_config_json());
    } else {
        result = kreuzberg_extract_file_sync(path);
    }

    return result;
}

/* -------------------------------------------------------------------------- */
/* Modes                                                                       */
/* -------------------------------------------------------------------------- */

static int run_server(bool ocr_enabled) {
    DEBUG_LOG("Server mode: reading paths from stdin");
    char line[8192];

    /* Signal readiness */
    printf("READY\n");
    fflush(stdout);

    while (fgets(line, sizeof(line), stdin) != NULL) {
        const char *path = NULL;
        bool force_ocr = false;
        parse_request(line, &path, &force_ocr);

        if (!path || strlen(path) == 0)
            continue;

        DEBUG_LOG("Extracting: %s (force_ocr=%d)", path, force_ocr);

        double start = time_ms();
        struct CExtractionResult *result = extract_file(path, ocr_enabled, force_ocr);
        double elapsed = time_ms() - start;

        if (!result || !result->success) {
            const char *err = kreuzberg_last_error();
            DEBUG_LOG("Extraction failed: %s", err ? err : "unknown");
            print_error_json(err);
            if (result)
                kreuzberg_free_result(result);
            continue;
        }

        bool ocr_used = determine_ocr_used(result->mime_type, ocr_enabled || force_ocr);
        print_result_json(result, elapsed, ocr_used);
        printf("\n");
        fflush(stdout);

        kreuzberg_free_result(result);
    }

    return 0;
}

static int run_sync(const char *path, bool ocr_enabled) {
    DEBUG_LOG("Sync mode: extracting %s", path);

    double start = time_ms();
    struct CExtractionResult *result = extract_file(path, ocr_enabled, false);
    double elapsed = time_ms() - start;

    if (!result || !result->success) {
        const char *err = kreuzberg_last_error();
        fprintf(stderr, "Error extracting with C binding: %s\n", err ? err : "unknown");
        if (result)
            kreuzberg_free_result(result);
        return 1;
    }

    bool ocr_used = determine_ocr_used(result->mime_type, ocr_enabled);
    print_result_json(result, elapsed, ocr_used);
    printf("\n");
    fflush(stdout);

    kreuzberg_free_result(result);
    return 0;
}

static int run_batch(int file_count, const char **files, bool ocr_enabled) {
    DEBUG_LOG("Batch mode: extracting %d files", file_count);

    double batch_start = time_ms();

    if (file_count == 1) {
        /* Single file in batch mode: return single object (not array) */
        double start = time_ms();
        struct CExtractionResult *result = extract_file(files[0], ocr_enabled, false);
        double elapsed = time_ms() - start;
        double total = time_ms() - batch_start;

        if (!result || !result->success) {
            const char *err = kreuzberg_last_error();
            fprintf(stderr, "Error extracting with C binding: %s\n", err ? err : "unknown");
            if (result)
                kreuzberg_free_result(result);
            return 1;
        }

        bool ocr_used = determine_ocr_used(result->mime_type, ocr_enabled);
        /* Include _batch_total_ms for batch mode */
        uint64_t mem = peak_memory_bytes();
        char *escaped_content = result->content ? json_escape_alloc(result->content) : NULL;
        const char *content_str = escaped_content ? escaped_content : "";
        const char *metadata_str = result->metadata_json ? result->metadata_json : "{}";

        printf("{\"content\":\"%s\",\"metadata\":%s,"
               "\"_extraction_time_ms\":%.2f,"
               "\"_batch_total_ms\":%.2f,"
               "\"_ocr_used\":%s,"
               "\"_peak_memory_bytes\":%llu}",
               content_str, metadata_str, elapsed, total, ocr_used ? "true" : "false",
               (unsigned long long)mem);
        printf("\n");
        fflush(stdout);
        free(escaped_content);
        kreuzberg_free_result(result);
        return 0;
    }

    /* Multiple files: print JSON array */
    printf("[");
    for (int i = 0; i < file_count; i++) {
        struct CExtractionResult *result = extract_file(files[i], ocr_enabled, false);
        double total = time_ms() - batch_start;

        if (i > 0)
            printf(",");

        if (!result || !result->success) {
            const char *err = kreuzberg_last_error();
            char *escaped = err ? json_escape_alloc(err) : NULL;
            printf("{\"error\":\"%s\",\"_extraction_time_ms\":0,"
                   "\"_batch_total_ms\":%.2f,\"_ocr_used\":false}",
                   escaped ? escaped : "unknown", total);
            free(escaped);
            if (result)
                kreuzberg_free_result(result);
            continue;
        }

        bool ocr_used = determine_ocr_used(result->mime_type, ocr_enabled);
        uint64_t mem = peak_memory_bytes();
        char *escaped_content = result->content ? json_escape_alloc(result->content) : NULL;
        const char *content_str = escaped_content ? escaped_content : "";
        const char *metadata_str = result->metadata_json ? result->metadata_json : "{}";

        double per_ms = total / (double)(file_count > 0 ? file_count : 1);

        printf("{\"content\":\"%s\",\"metadata\":%s,"
               "\"_extraction_time_ms\":%.2f,"
               "\"_batch_total_ms\":%.2f,"
               "\"_ocr_used\":%s,"
               "\"_peak_memory_bytes\":%llu}",
               content_str, metadata_str, per_ms, total, ocr_used ? "true" : "false",
               (unsigned long long)mem);
        free(escaped_content);
        kreuzberg_free_result(result);
    }
    printf("]\n");
    fflush(stdout);
    return 0;
}

/* -------------------------------------------------------------------------- */
/* Main                                                                        */
/* -------------------------------------------------------------------------- */

int main(int argc, char *argv[]) {
    debug_enabled = getenv("KREUZBERG_BENCHMARK_DEBUG") != NULL &&
                    strlen(getenv("KREUZBERG_BENCHMARK_DEBUG")) > 0;

    DEBUG_LOG("Kreuzberg C extraction script started");

    bool ocr_enabled = false;
    const char *mode = NULL;
    const char **files = NULL;
    int file_count = 0;

    /* Parse arguments */
    int arg_idx = 1;
    while (arg_idx < argc) {
        if (strcmp(argv[arg_idx], "--ocr") == 0) {
            ocr_enabled = true;
        } else if (strcmp(argv[arg_idx], "--no-ocr") == 0) {
            ocr_enabled = false;
        } else if (!mode) {
            mode = argv[arg_idx];
        } else {
            /* Remaining args are file paths */
            files = (const char **)&argv[arg_idx];
            file_count = argc - arg_idx;
            break;
        }
        arg_idx++;
    }

    if (!mode) {
        fprintf(stderr, "Usage: kreuzberg_extract_c [--ocr|--no-ocr] <mode> "
                        "[file_paths...]\n"
                        "Modes: sync, batch, server\n");
        return 1;
    }

    DEBUG_LOG("Mode: %s, OCR enabled: %d", mode, ocr_enabled);

    if (strcmp(mode, "server") == 0) {
        return run_server(ocr_enabled);
    } else if (strcmp(mode, "sync") == 0) {
        if (file_count != 1) {
            fprintf(stderr, "sync mode requires exactly one file\n");
            return 1;
        }
        return run_sync(files[0], ocr_enabled);
    } else if (strcmp(mode, "batch") == 0) {
        if (file_count == 0) {
            fprintf(stderr, "batch mode requires at least one file\n");
            return 1;
        }
        return run_batch(file_count, files, ocr_enabled);
    } else {
        fprintf(stderr, "Unknown mode: %s\n", mode);
        return 1;
    }
}
