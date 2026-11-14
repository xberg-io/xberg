#include <stdint.h>
#include <stdlib.h>
#include <string.h>
typedef uint64_t H;
static int mode, frees, configs, calls, stale, metadata_frees, byte_frees;
void set_mode(int m) {
  mode = m;
  frees = configs = calls = stale = metadata_frees = byte_frees = 0;
}
int receipts(void) {
  return frees + 10 * configs + 100 * calls + 100000 * stale +
         1000 * metadata_frees + 10000 * byte_frees;
}
H xberg_extract_input_from_json_with_bytes(void *bytes, size_t len,
                                           const char *meta) {
  return 11;
}
H xberg_extraction_config_from_json(const char *json) {
  return mode == 0 ? 0 : 22;
}
void xberg_extract_input_free(H input) {
  if (input == 11 && mode != 2 && mode < 5)
    frees++;
  else
    stale++;
}
void xberg_extraction_config_free(H config) {
  if (config == 22)
    configs++;
  else
    stale++;
}
H xberg_extract_with_layout_pages_owned(H *input, H config, const char *pages) {
  if (*input != 11 || config != 22 || !pages)
    abort();
  calls++;
  if (mode == 2)
    *input = 0;
  return 0;
}
int xberg_last_error_code(void) { return 1; }
void *xberg_last_error_context(void) { return 0; }
H xberg_extract_input_from_bytes(const uint8_t *bytes, size_t len,
                                 const char *mime, const char *filename) {
  const uint8_t expected[] = {0, 128, 255};
  if (len != sizeof(expected) || memcmp(bytes, expected, len) ||
      strcmp(mime, "probe/type") || strcmp(filename, "probe.bin"))
    abort();
  return mode == 3 ? 0 : 11;
}
char *xberg_extract_input_alef_take_factory_bytes(H *input, uint8_t **bytes,
                                                  size_t *len) {
  if (*input != 11 || (mode != 4 && mode != 5))
    abort();
  calls++;
  if (mode == 4)
    return NULL;
  *input = 0;
  *len = 3;
  *bytes = malloc(*len);
  return strdup("{");
}
void xberg_free_string(char *metadata) {
  metadata_frees++;
  free(metadata);
}
void xberg_free_bytes(uint8_t *bytes, size_t len, size_t cap) {
  if (len != 3 || cap != 3)
    abort();
  byte_frees++;
  free(bytes);
}
