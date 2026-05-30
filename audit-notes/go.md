# Go Binding Systematic Bug Audit — May 30, 2026

## BINDING_BUG: Handle Lifetime Management in Trait Bridge Callbacks

### Issue
When trait implementations (DocumentExtractor, OcrBackend, EmbeddingBackend, PostProcessor, Renderer, Validator) are registered via `Register*()` functions, the Go `cgo.Handle` is created and passed to Rust as `userData`. However, when `Unregister*()` is called, **the Go handle is NEVER deleted**, causing use-after-free crashes when:

1. First extractor registered → handle stored in Go's cgo handle table
2. Extractor registered again → Rust still holds vtable to first implementation
3. Second test calls the first extractor's methods → crashes with SIGBUS on callback

### Root Cause
- `RegisterDocumentExtractor()` calls `handle.Delete()` only on **registration error** (line 1616), not on success
- `UnregisterDocumentExtractor()` has **no way to delete the handle** because it doesn't track which handle name corresponds to
- All trait bridge exports (`goDocumentExtractorPriority`, `goDocumentExtractorCanHandle`, etc.) dereference `userData` as a `cgo.Handle` without validation

### Evidence
**Stack trace** (test: `Test_RegisterDocumentExtractorTraitBridge`):
```
panic: runtime error: cgo/go interop SIGBUS at 0x1043999ac
...
github.com/kreuzberg-dev/kreuzberg/v5.goDocumentExtractorPriority(0x..., 0x1043999ac, ...) // <- bad pointer
  packages/go/v5/trait_bridges.go:1476 in *outResult = cResult
```

**Root cause**: Handle was deleted in between callback invocations, or pointer became invalid.

### Affected Functions
All trait bridge exports in `packages/go/v5/trait_bridges.go`:

**DocumentExtractor** (11 callbacks):
- goDocumentExtractorName, goDocumentExtractorVersion, goDocumentExtractorDescription, goDocumentExtractorAuthor, goDocumentExtractorInitialize, goDocumentExtractorShutdown, goDocumentExtractorPriority, goDocumentExtractorCanHandle, goDocumentExtractorSupportedMimeTypes, goDocumentExtractorExtractBytes, goDocumentExtractorExtractFile

**OcrBackend** (10 callbacks):
- goOcrBackendName, goOcrBackendVersion, goOcrBackendDescription, goOcrBackendAuthor, goOcrBackendInitialize, goOcrBackendShutdown, goOcrBackendProcessImage, goOcrBackendProcessImageFile, goOcrBackendSupportsLanguage, goOcrBackendBackendType, goOcrBackendSupportedLanguages, goOcrBackendSupportsTableDetection, goOcrBackendSupportsDocumentProcessing, goOcrBackendProcessDocument

**EmbeddingBackend** (8 callbacks):
- goEmbeddingBackendName, goEmbeddingBackendVersion, goEmbeddingBackendDescription, goEmbeddingBackendAuthor, goEmbeddingBackendInitialize, goEmbeddingBackendShutdown, goEmbeddingBackendDimensions, goEmbeddingBackendEmbed

**PostProcessor** (6 callbacks):
- goPostProcessorName, goPostProcessorVersion, goPostProcessorDescription, goPostProcessorAuthor, goPostProcessorInitialize, goPostProcessorShutdown, goPostProcessorProcessingStage, goPostProcessorShouldProcess, goPostProcessorEstimatedDurationMs, goPostProcessorPriority, goPostProcessorProcess

**Renderer** (4 callbacks):
- goRendererName, goRendererVersion, goRendererDescription, goRendererAuthor, goRendererInitialize, goRendererShutdown, goRendererRender

**Validator** (6 callbacks):
- goValidatorName, goValidatorVersion, goValidatorDescription, goValidatorAuthor, goValidatorInitialize, goValidatorShutdown, goValidatorPriority, goValidatorShouldValidate, goValidatorValidate

### Fix Strategy

**1. Track handles in a global map** (implementation.go):
```go
var (
    extractorHandles = make(map[string]cgo.Handle)  // name -> handle
    ocrHandles       = make(map[string]cgo.Handle)
    embeddingHandles = make(map[string]cgo.Handle)
    postProcessorHandles = make(map[string]cgo.Handle)
    rendererHandles  = make(map[string]cgo.Handle)
    validatorHandles = make(map[string]cgo.Handle)
    handlesMu        sync.Mutex  // protect map access
)
```

**2. Store handle on register:**
```go
func RegisterDocumentExtractor(impl DocumentExtractor) error {
    handle := cgo.NewHandle(impl)
    name := impl.Name()

    handlesMu.Lock()
    extractorHandles[name] = handle
    handlesMu.Unlock()

    // register with Rust...
    if err != nil {
        handlesMu.Lock()
        delete(extractorHandles, name)
        handlesMu.Unlock()
        handle.Delete()
        return err
    }
}
```

**3. Delete handle on unregister:**
```go
func UnregisterDocumentExtractor(name string) error {
    // unregister from Rust first...
    if err != nil {
        return err
    }

    handlesMu.Lock()
    if handle, ok := extractorHandles[name]; ok {
        delete(extractorHandles, name)
        handle.Delete()
    }
    handlesMu.Unlock()
}
```

**4. Clear all handles on clear operations:**
```go
func ClearDocumentExtractors() error {
    // clear from Rust...
    if err != nil {
        return err
    }

    handlesMu.Lock()
    for _, handle := range extractorHandles {
        handle.Delete()
    }
    extractorHandles = make(map[string]cgo.Handle)
    handlesMu.Unlock()
}
```

### Secondary Issues Found

**MEMORY_LEAK**: C.CString allocations in trait bridge callbacks
- Functions like `goDocumentExtractorName()` at line 125 do:
  ```go
  cName := C.CString(name)
  *outResult = cName
  ```
- The string is allocated by `C.CString()` but Rust must free it after reading
- **Status**: This is expected design (Rust owns the result pointer and must free), but worth documenting

**ERROR_HANDLING**: Missing error propagation in callbacks
- If `json.Marshal()` fails in trait bridge callback (line 121), the error is silently dropped
- Result is empty JSON "null" or "{}", causing semantic issues
- **Fix**: Check marshal error and return via outError pointer

### Testing Recommendations

1. **Handle leak detection**: Run tests with `-race` and monitor for SIGBUS
2. **Concurrent registration**: Register/unregister same trait in parallel threads
3. **Lifecycle sequence**: Register → use → unregister → verify no SIGBUS on later operations
4. **GC pressure**: Force GC between register/unregister to catch use-after-free

### Compliance

- **cgo memory ownership**: Every handle creation must have corresponding deletion
- **unsafe.Pointer lifetime**: userData pointer must remain valid for handle's entire lifetime
- **Concurrency**: Map access must be protected with sync.Mutex
