#include <tesseract/capi.h>

extern "C" {

int xberg_tess_recognize(void *handle, void *monitor) {
  try {
    return TessBaseAPIRecognize(static_cast<TessBaseAPI *>(handle),
                                static_cast<ETEXT_DESC *>(monitor));
  } catch (...) {
    return -1;
  }
}

char *xberg_tess_get_hocr_text(void *handle, int page) {
  try {
    return TessBaseAPIGetHOCRText(static_cast<TessBaseAPI *>(handle), page);
  } catch (...) {
    return nullptr;
  }
}

char *xberg_tess_get_utf8_text(void *handle) {
  try {
    return TessBaseAPIGetUTF8Text(static_cast<TessBaseAPI *>(handle));
  } catch (...) {
    return nullptr;
  }
}

int xberg_tess_clear(void *handle) {
  if (handle == nullptr) {
    return -1;
  }
  try {
    TessBaseAPIClear(static_cast<TessBaseAPI *>(handle));
    return 0;
  } catch (...) {
    return -1;
  }
}

int xberg_tess_detect_orientation_script(void *handle, int *orient_deg,
                                         float *orient_conf,
                                         const char **script_name,
                                         float *script_conf) {
  try {
    return TessBaseAPIDetectOrientationScript(
        static_cast<TessBaseAPI *>(handle), orient_deg, orient_conf,
        script_name, script_conf);
  } catch (...) {
    return 0;
  }
}
}
