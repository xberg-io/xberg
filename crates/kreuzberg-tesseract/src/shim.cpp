// C++ exception barrier shims for Tesseract C API functions that can propagate
// C++ exceptions into Rust FFI frames. Each function wraps the raw Tesseract call
// in try/catch (...) so no exception can escape into the Rust call stack.
//
// Without these shims, a C++ exception escaping through extern "C-unwind" frames
// causes the Rust runtime to abort with "Rust cannot catch foreign exceptions".
//
// These shims are compiled with exception support enabled (no -fno-exceptions),
// so catch (...) is active even when the wrapped Tesseract library was built
// with -fno-exceptions (where exceptions from the C++ stdlib can still propagate
// through Tesseract frames into ours).
#include <tesseract/capi.h>

extern "C" {

int kreuzberg_tess_recognize(void* handle) {
    try {
        return TessBaseAPIRecognize(static_cast<TessBaseAPI*>(handle), nullptr);
    } catch (...) {
        return -1;
    }
}

char* kreuzberg_tess_get_hocr_text(void* handle, int page) {
    try {
        return TessBaseAPIGetHOCRText(static_cast<TessBaseAPI*>(handle), page);
    } catch (...) {
        return nullptr;
    }
}

char* kreuzberg_tess_get_utf8_text(void* handle) {
    try {
        return TessBaseAPIGetUTF8Text(static_cast<TessBaseAPI*>(handle));
    } catch (...) {
        return nullptr;
    }
}

void kreuzberg_tess_clear(void* handle) {
    try {
        TessBaseAPIClear(static_cast<TessBaseAPI*>(handle));
    } catch (...) {}
}

int kreuzberg_tess_detect_orientation_script(
    void* handle,
    int* orient_deg,
    float* orient_conf,
    char** script_name,
    float* script_conf
) {
    try {
        // Tesseract's C API declares script_name as const char** (output read-only to caller).
        // Rust passes *mut *mut c_char (char**); add const with a C-style cast — safe because
        // we only add const, we do not remove it.
        return TessBaseAPIDetectOrientationScript(
            static_cast<TessBaseAPI*>(handle),
            orient_deg,
            orient_conf,
            (const char**)script_name,
            script_conf
        );
    } catch (...) {
        return 0;
    }
}

} // extern "C"
