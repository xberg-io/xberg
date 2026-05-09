import RustBridgeC
public func extractBytes<GenericIntoRustString: IntoRustString>(_ content: RustVec<UInt8>, _ mime_type: GenericIntoRustString, _ config: ExtractionConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$extract_bytes({ let val = content; val.isOwned = false; return val.ptr }(), { let rustString = mime_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func extractFile<GenericIntoRustString: IntoRustString>(_ path: GenericIntoRustString, _ mime_type: Optional<GenericIntoRustString>, _ config: ExtractionConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$extract_file({ let rustString = path.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let rustString = optionalStringIntoRustString(mime_type) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func extractFileSync<GenericIntoRustString: IntoRustString>(_ path: GenericIntoRustString, _ mime_type: Optional<GenericIntoRustString>, _ config: ExtractionConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$extract_file_sync({ let rustString = path.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let rustString = optionalStringIntoRustString(mime_type) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func extractBytesSync<GenericIntoRustString: IntoRustString>(_ content: RustVec<UInt8>, _ mime_type: GenericIntoRustString, _ config: ExtractionConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$extract_bytes_sync({ let val = content; val.isOwned = false; return val.ptr }(), { let rustString = mime_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func batchExtractFilesSync(_ items: RustVec<BatchFileItem>, _ config: ExtractionConfig) throws -> RustVec<ExtractionResult> {
    try { let val = __swift_bridge__$batch_extract_files_sync({ let val = items; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func batchExtractBytesSync(_ items: RustVec<BatchBytesItem>, _ config: ExtractionConfig) throws -> RustVec<ExtractionResult> {
    try { let val = __swift_bridge__$batch_extract_bytes_sync({ let val = items; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func batchExtractFiles(_ items: RustVec<BatchFileItem>, _ config: ExtractionConfig) throws -> RustVec<ExtractionResult> {
    try { let val = __swift_bridge__$batch_extract_files({ let val = items; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func batchExtractBytes(_ items: RustVec<BatchBytesItem>, _ config: ExtractionConfig) throws -> RustVec<ExtractionResult> {
    try { let val = __swift_bridge__$batch_extract_bytes({ let val = items; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func detectMimeTypeFromBytes(_ content: RustVec<UInt8>) throws -> RustString {
    try { let val = __swift_bridge__$detect_mime_type_from_bytes({ let val = content; val.isOwned = false; return val.ptr }()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func getExtensionsForMime<GenericIntoRustString: IntoRustString>(_ mime_type: GenericIntoRustString) throws -> RustVec<RustString> {
    try { let val = __swift_bridge__$get_extensions_for_mime({ let rustString = mime_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func listDocumentExtractors() throws -> RustVec<RustString> {
    try { let val = __swift_bridge__$list_document_extractors(); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func listOcrBackends() throws -> RustVec<RustString> {
    try { let val = __swift_bridge__$list_ocr_backends(); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func clearOcrBackends() throws -> () {
    try { let val = __swift_bridge__$clear_ocr_backends(); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func listPostProcessors() throws -> RustVec<RustString> {
    try { let val = __swift_bridge__$list_post_processors(); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func clearPostProcessors() throws -> () {
    try { let val = __swift_bridge__$clear_post_processors(); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func listValidators() throws -> RustVec<RustString> {
    try { let val = __swift_bridge__$list_validators(); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func clearValidators() throws -> () {
    try { let val = __swift_bridge__$clear_validators(); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func embedTextsAsync<GenericIntoRustString: IntoRustString>(_ texts: RustVec<GenericIntoRustString>, _ config: EmbeddingConfig) throws -> RustString {
    try { let val = __swift_bridge__$embed_texts_async({ let val = texts; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func renderPdfPageToPng<GenericIntoRustString: IntoRustString>(_ pdf_bytes: RustVec<UInt8>, _ page_index: UInt, _ dpi: Optional<Int32>, _ password: Optional<GenericIntoRustString>) throws -> RustVec<UInt8> {
    try { let val = __swift_bridge__$render_pdf_page_to_png({ let val = pdf_bytes; val.isOwned = false; return val.ptr }(), page_index, dpi.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(password) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()); if val.is_ok { return RustVec(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func detectMimeType<GenericIntoRustString: IntoRustString>(_ path: GenericIntoRustString, _ check_exists: Bool) throws -> RustString {
    try { let val = __swift_bridge__$detect_mime_type({ let rustString = path.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), check_exists); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func embedTexts<GenericIntoRustString: IntoRustString>(_ texts: RustVec<GenericIntoRustString>, _ config: EmbeddingConfig) throws -> RustString {
    try { let val = __swift_bridge__$embed_texts({ let val = texts; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func getEmbeddingPreset<GenericIntoRustString: IntoRustString>(_ name: GenericIntoRustString) -> RustString {
    RustString(ptr: __swift_bridge__$get_embedding_preset({ let rustString = name.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
}
public func listEmbeddingPresets() -> RustVec<RustString> {
    RustVec(ptr: __swift_bridge__$list_embedding_presets())
}
public func alef_phantom_vec_ocr_backend() -> RustVec<OcrBackendBox> {
    RustVec(ptr: __swift_bridge__$alef_phantom_vec_ocr_backend())
}
public func ocr_backend_call_process_image(_ this: OcrBackendBoxRef, _ image_bytes: RustVec<UInt8>, _ config: OcrConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$ocr_backend_call_process_image(this.ptr, { let val = image_bytes; val.isOwned = false; return val.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func ocr_backend_call_process_image_file<GenericIntoRustString: IntoRustString>(_ this: OcrBackendBoxRef, _ path: GenericIntoRustString, _ config: OcrConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$ocr_backend_call_process_image_file(this.ptr, { let rustString = path.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func ocr_backend_call_supports_language<GenericIntoRustString: IntoRustString>(_ this: OcrBackendBoxRef, _ lang: GenericIntoRustString) -> Bool {
    __swift_bridge__$ocr_backend_call_supports_language(this.ptr, { let rustString = lang.intoRustString(); rustString.isOwned = false; return rustString.ptr }())
}
public func ocr_backend_call_backend_type(_ this: OcrBackendBoxRef) -> OcrBackendType {
    OcrBackendType(ptr: __swift_bridge__$ocr_backend_call_backend_type(this.ptr))
}
public func ocr_backend_call_supported_languages(_ this: OcrBackendBoxRef) -> RustVec<RustString> {
    RustVec(ptr: __swift_bridge__$ocr_backend_call_supported_languages(this.ptr))
}
public func ocr_backend_call_supports_table_detection(_ this: OcrBackendBoxRef) -> Bool {
    __swift_bridge__$ocr_backend_call_supports_table_detection(this.ptr)
}
public func ocr_backend_call_supports_document_processing(_ this: OcrBackendBoxRef) -> Bool {
    __swift_bridge__$ocr_backend_call_supports_document_processing(this.ptr)
}
public func ocr_backend_call_process_document<GenericIntoRustString: IntoRustString>(_ this: OcrBackendBoxRef, _ path: GenericIntoRustString, _ config: OcrConfig) throws -> ExtractionResult {
    try { let val = __swift_bridge__$ocr_backend_call_process_document(this.ptr, { let rustString = path.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {config.isOwned = false; return config.ptr;}()); if val.is_ok { return ExtractionResult(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func alef_phantom_vec_post_processor() -> RustVec<PostProcessorBox> {
    RustVec(ptr: __swift_bridge__$alef_phantom_vec_post_processor())
}
public func post_processor_call_process(_ this: PostProcessorBoxRef, _ result: ExtractionResult, _ config: ExtractionConfig) throws -> () {
    try { let val = __swift_bridge__$post_processor_call_process(this.ptr, {result.isOwned = false; return result.ptr;}(), {config.isOwned = false; return config.ptr;}()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func post_processor_call_processing_stage(_ this: PostProcessorBoxRef) -> ProcessingStage {
    ProcessingStage(ptr: __swift_bridge__$post_processor_call_processing_stage(this.ptr))
}
public func post_processor_call_should_process(_ this: PostProcessorBoxRef, _ result: ExtractionResult, _ config: ExtractionConfig) -> Bool {
    __swift_bridge__$post_processor_call_should_process(this.ptr, {result.isOwned = false; return result.ptr;}(), {config.isOwned = false; return config.ptr;}())
}
public func post_processor_call_estimated_duration_ms(_ this: PostProcessorBoxRef, _ result: ExtractionResult) -> UInt64 {
    __swift_bridge__$post_processor_call_estimated_duration_ms(this.ptr, {result.isOwned = false; return result.ptr;}())
}
public func post_processor_call_priority(_ this: PostProcessorBoxRef) -> Int32 {
    __swift_bridge__$post_processor_call_priority(this.ptr)
}
public func alef_phantom_vec_validator() -> RustVec<ValidatorBox> {
    RustVec(ptr: __swift_bridge__$alef_phantom_vec_validator())
}
public func validator_call_validate(_ this: ValidatorBoxRef, _ result: ExtractionResult, _ config: ExtractionConfig) throws -> () {
    try { let val = __swift_bridge__$validator_call_validate(this.ptr, {result.isOwned = false; return result.ptr;}(), {config.isOwned = false; return config.ptr;}()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func validator_call_should_validate(_ this: ValidatorBoxRef, _ result: ExtractionResult, _ config: ExtractionConfig) -> Bool {
    __swift_bridge__$validator_call_should_validate(this.ptr, {result.isOwned = false; return result.ptr;}(), {config.isOwned = false; return config.ptr;}())
}
public func validator_call_priority(_ this: ValidatorBoxRef) -> Int32 {
    __swift_bridge__$validator_call_priority(this.ptr)
}
public func alef_phantom_vec_embedding_backend() -> RustVec<EmbeddingBackendBox> {
    RustVec(ptr: __swift_bridge__$alef_phantom_vec_embedding_backend())
}
public func embedding_backend_call_dimensions(_ this: EmbeddingBackendBoxRef) -> UInt {
    __swift_bridge__$embedding_backend_call_dimensions(this.ptr)
}
public func embedding_backend_call_embed<GenericIntoRustString: IntoRustString>(_ this: EmbeddingBackendBoxRef, _ texts: RustVec<GenericIntoRustString>) throws -> RustString {
    try { let val = __swift_bridge__$embedding_backend_call_embed(this.ptr, { let val = texts; val.isOwned = false; return val.ptr }()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func registerOcrBackend(_ swift_box: SwiftOcrBackendBox) throws -> () {
    try { let val = __swift_bridge__$register_ocr_backend(Unmanaged.passRetained(swift_box).toOpaque()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func registerPostProcessor(_ swift_box: SwiftPostProcessorBox) throws -> () {
    try { let val = __swift_bridge__$register_post_processor(Unmanaged.passRetained(swift_box).toOpaque()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func registerValidator(_ swift_box: SwiftValidatorBox) throws -> () {
    try { let val = __swift_bridge__$register_validator(Unmanaged.passRetained(swift_box).toOpaque()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
public func registerEmbeddingBackend(_ swift_box: SwiftEmbeddingBackendBox) throws -> () {
    try { let val = __swift_bridge__$register_embedding_backend(Unmanaged.passRetained(swift_box).toOpaque()); if val != nil { throw RustString(ptr: val!) } else { return } }()
}
@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_name")
func __swift_bridge__SwiftOcrBackendBox_alef_name (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_name().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_version")
func __swift_bridge__SwiftOcrBackendBox_alef_version (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_version().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_initialize")
func __swift_bridge__SwiftOcrBackendBox_alef_initialize (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_initialize().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_shutdown")
func __swift_bridge__SwiftOcrBackendBox_alef_shutdown (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_shutdown().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_process_image")
func __swift_bridge__SwiftOcrBackendBox_alef_process_image (_ this: UnsafeMutableRawPointer, _ image_bytes: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_process_image(image_bytes: RustVec(ptr: image_bytes), config: RustString(ptr: config)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_process_image_file")
func __swift_bridge__SwiftOcrBackendBox_alef_process_image_file (_ this: UnsafeMutableRawPointer, _ path: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_process_image_file(path: RustString(ptr: path), config: RustString(ptr: config)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_supports_language")
func __swift_bridge__SwiftOcrBackendBox_alef_supports_language (_ this: UnsafeMutableRawPointer, _ lang: UnsafeMutableRawPointer) -> Bool {
    Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_supports_language(lang: RustString(ptr: lang))
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_backend_type")
func __swift_bridge__SwiftOcrBackendBox_alef_backend_type (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_backend_type().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_supported_languages")
func __swift_bridge__SwiftOcrBackendBox_alef_supported_languages (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let val = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_supported_languages(); val.isOwned = false; return val.ptr }()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_supports_table_detection")
func __swift_bridge__SwiftOcrBackendBox_alef_supports_table_detection (_ this: UnsafeMutableRawPointer) -> Bool {
    Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_supports_table_detection()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_supports_document_processing")
func __swift_bridge__SwiftOcrBackendBox_alef_supports_document_processing (_ this: UnsafeMutableRawPointer) -> Bool {
    Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_supports_document_processing()
}

@_cdecl("__swift_bridge__$SwiftOcrBackendBox$alef_process_document")
func __swift_bridge__SwiftOcrBackendBox_alef_process_document (_ this: UnsafeMutableRawPointer, _ path: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftOcrBackendBox>.fromOpaque(this).takeUnretainedValue().alef_process_document(path: RustString(ptr: path), config: RustString(ptr: config)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_name")
func __swift_bridge__SwiftPostProcessorBox_alef_name (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_name().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_version")
func __swift_bridge__SwiftPostProcessorBox_alef_version (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_version().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_initialize")
func __swift_bridge__SwiftPostProcessorBox_alef_initialize (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_initialize().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_shutdown")
func __swift_bridge__SwiftPostProcessorBox_alef_shutdown (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_shutdown().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_process")
func __swift_bridge__SwiftPostProcessorBox_alef_process (_ this: UnsafeMutableRawPointer, _ result: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_process(result: RustString(ptr: result), config: RustString(ptr: config)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_processing_stage")
func __swift_bridge__SwiftPostProcessorBox_alef_processing_stage (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_processing_stage().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_should_process")
func __swift_bridge__SwiftPostProcessorBox_alef_should_process (_ this: UnsafeMutableRawPointer, _ result: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> Bool {
    Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_should_process(result: RustString(ptr: result), config: RustString(ptr: config))
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_estimated_duration_ms")
func __swift_bridge__SwiftPostProcessorBox_alef_estimated_duration_ms (_ this: UnsafeMutableRawPointer, _ result: UnsafeMutableRawPointer) -> UInt64 {
    Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_estimated_duration_ms(result: RustString(ptr: result))
}

@_cdecl("__swift_bridge__$SwiftPostProcessorBox$alef_priority")
func __swift_bridge__SwiftPostProcessorBox_alef_priority (_ this: UnsafeMutableRawPointer) -> Int32 {
    Unmanaged<SwiftPostProcessorBox>.fromOpaque(this).takeUnretainedValue().alef_priority()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_name")
func __swift_bridge__SwiftValidatorBox_alef_name (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_name().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_version")
func __swift_bridge__SwiftValidatorBox_alef_version (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_version().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_initialize")
func __swift_bridge__SwiftValidatorBox_alef_initialize (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_initialize().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_shutdown")
func __swift_bridge__SwiftValidatorBox_alef_shutdown (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_shutdown().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_validate")
func __swift_bridge__SwiftValidatorBox_alef_validate (_ this: UnsafeMutableRawPointer, _ result: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_validate(result: RustString(ptr: result), config: RustString(ptr: config)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_should_validate")
func __swift_bridge__SwiftValidatorBox_alef_should_validate (_ this: UnsafeMutableRawPointer, _ result: UnsafeMutableRawPointer, _ config: UnsafeMutableRawPointer) -> Bool {
    Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_should_validate(result: RustString(ptr: result), config: RustString(ptr: config))
}

@_cdecl("__swift_bridge__$SwiftValidatorBox$alef_priority")
func __swift_bridge__SwiftValidatorBox_alef_priority (_ this: UnsafeMutableRawPointer) -> Int32 {
    Unmanaged<SwiftValidatorBox>.fromOpaque(this).takeUnretainedValue().alef_priority()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_name")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_name (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_name().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_version")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_version (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_version().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_initialize")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_initialize (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_initialize().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_shutdown")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_shutdown (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_shutdown().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_dimensions")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_dimensions (_ this: UnsafeMutableRawPointer) -> UInt {
    Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_dimensions()
}

@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$alef_embed")
func __swift_bridge__SwiftEmbeddingBackendBox_alef_embed (_ this: UnsafeMutableRawPointer, _ texts: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    { let rustString = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(this).takeUnretainedValue().alef_embed(texts: RustVec(ptr: texts)).intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}


public class AccelerationConfig: AccelerationConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$AccelerationConfig$_free(ptr)
        }
    }
}
extension AccelerationConfig {
    public convenience init(_ provider: ExecutionProviderType, _ device_id: UInt32) {
        self.init(ptr: __swift_bridge__$AccelerationConfig$new({provider.isOwned = false; return provider.ptr;}(), device_id))
    }
}
public class AccelerationConfigRefMut: AccelerationConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class AccelerationConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension AccelerationConfigRef {
    public func provider() -> ExecutionProviderType {
        ExecutionProviderType(ptr: __swift_bridge__$AccelerationConfig$provider(ptr))
    }

    public func device_id() -> UInt32 {
        __swift_bridge__$AccelerationConfig$device_id(ptr)
    }
}
extension AccelerationConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_AccelerationConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_AccelerationConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: AccelerationConfig) {
        __swift_bridge__$Vec_AccelerationConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_AccelerationConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (AccelerationConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AccelerationConfigRef> {
        let pointer = __swift_bridge__$Vec_AccelerationConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AccelerationConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AccelerationConfigRefMut> {
        let pointer = __swift_bridge__$Vec_AccelerationConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AccelerationConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<AccelerationConfigRef> {
        UnsafePointer<AccelerationConfigRef>(OpaquePointer(__swift_bridge__$Vec_AccelerationConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_AccelerationConfig$len(vecPtr)
    }
}


public class ContentFilterConfig: ContentFilterConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ContentFilterConfig$_free(ptr)
        }
    }
}
extension ContentFilterConfig {
    public convenience init(_ include_headers: Bool, _ include_footers: Bool, _ strip_repeating_text: Bool, _ include_watermarks: Bool) {
        self.init(ptr: __swift_bridge__$ContentFilterConfig$new(include_headers, include_footers, strip_repeating_text, include_watermarks))
    }
}
public class ContentFilterConfigRefMut: ContentFilterConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ContentFilterConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ContentFilterConfigRef {
    public func include_headers() -> Bool {
        __swift_bridge__$ContentFilterConfig$include_headers(ptr)
    }

    public func include_footers() -> Bool {
        __swift_bridge__$ContentFilterConfig$include_footers(ptr)
    }

    public func strip_repeating_text() -> Bool {
        __swift_bridge__$ContentFilterConfig$strip_repeating_text(ptr)
    }

    public func include_watermarks() -> Bool {
        __swift_bridge__$ContentFilterConfig$include_watermarks(ptr)
    }
}
extension ContentFilterConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ContentFilterConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ContentFilterConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ContentFilterConfig) {
        __swift_bridge__$Vec_ContentFilterConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ContentFilterConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ContentFilterConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContentFilterConfigRef> {
        let pointer = __swift_bridge__$Vec_ContentFilterConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContentFilterConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContentFilterConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ContentFilterConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContentFilterConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ContentFilterConfigRef> {
        UnsafePointer<ContentFilterConfigRef>(OpaquePointer(__swift_bridge__$Vec_ContentFilterConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ContentFilterConfig$len(vecPtr)
    }
}


public class EmailConfig: EmailConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmailConfig$_free(ptr)
        }
    }
}
extension EmailConfig {
    public convenience init(_ msg_fallback_codepage: Optional<UInt32>) {
        self.init(ptr: __swift_bridge__$EmailConfig$new(msg_fallback_codepage.intoFfiRepr()))
    }
}
public class EmailConfigRefMut: EmailConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmailConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmailConfigRef {
    public func msg_fallback_codepage() -> Optional<UInt32> {
        __swift_bridge__$EmailConfig$msg_fallback_codepage(ptr).intoSwiftRepr()
    }
}
extension EmailConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmailConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmailConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmailConfig) {
        __swift_bridge__$Vec_EmailConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmailConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmailConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailConfigRef> {
        let pointer = __swift_bridge__$Vec_EmailConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailConfigRefMut> {
        let pointer = __swift_bridge__$Vec_EmailConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmailConfigRef> {
        UnsafePointer<EmailConfigRef>(OpaquePointer(__swift_bridge__$Vec_EmailConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmailConfig$len(vecPtr)
    }
}


public class ExtractionConfig: ExtractionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractionConfig$_free(ptr)
        }
    }
}
extension ExtractionConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ use_cache: Bool, _ enable_quality_processing: Bool, _ ocr: Optional<OcrConfig>, _ force_ocr: Bool, _ force_ocr_pages: Optional<RustVec<UInt>>, _ disable_ocr: Bool, _ chunking: Optional<ChunkingConfig>, _ content_filter: Optional<ContentFilterConfig>, _ images: Optional<ImageExtractionConfig>, _ pdf_options: Optional<PdfConfig>, _ token_reduction: Optional<TokenReductionOptions>, _ language_detection: Optional<LanguageDetectionConfig>, _ pages: Optional<PageConfig>, _ keywords: Optional<KeywordConfig>, _ postprocessor: Optional<PostProcessorConfig>, _ html_options: Optional<GenericIntoRustString>, _ html_output: Optional<HtmlOutputConfig>, _ extraction_timeout_secs: Optional<UInt64>, _ max_concurrent_extractions: Optional<UInt>, _ result_format: ResultFormat, _ security_limits: Optional<SecurityLimits>, _ output_format: OutputFormat, _ layout: Optional<LayoutDetectionConfig>, _ use_layout_for_markdown: Bool, _ include_document_structure: Bool, _ acceleration: Optional<AccelerationConfig>, _ cache_namespace: Optional<GenericIntoRustString>, _ cache_ttl_secs: Optional<UInt64>, _ email: Optional<EmailConfig>, _ concurrency: Optional<GenericIntoRustString>, _ max_archive_depth: UInt, _ tree_sitter: Optional<TreeSitterConfig>, _ structured_extraction: Optional<StructuredExtractionConfig>, _ cancel_token: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$ExtractionConfig$new(use_cache, enable_quality_processing, { if let val = ocr { val.isOwned = false; return val.ptr } else { return nil } }(), force_ocr, { if let val = force_ocr_pages { val.isOwned = false; return val.ptr } else { return nil } }(), disable_ocr, { if let val = chunking { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = content_filter { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = images { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pdf_options { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = token_reduction { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = language_detection { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = keywords { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = postprocessor { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(html_options) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = html_output { val.isOwned = false; return val.ptr } else { return nil } }(), extraction_timeout_secs.intoFfiRepr(), max_concurrent_extractions.intoFfiRepr(), {result_format.isOwned = false; return result_format.ptr;}(), { if let val = security_limits { val.isOwned = false; return val.ptr } else { return nil } }(), {output_format.isOwned = false; return output_format.ptr;}(), { if let val = layout { val.isOwned = false; return val.ptr } else { return nil } }(), use_layout_for_markdown, include_document_structure, { if let val = acceleration { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(cache_namespace) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), cache_ttl_secs.intoFfiRepr(), { if let val = email { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(concurrency) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), max_archive_depth, { if let val = tree_sitter { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = structured_extraction { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(cancel_token) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class ExtractionConfigRefMut: ExtractionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractionConfigRef {
    public func use_cache() -> Bool {
        __swift_bridge__$ExtractionConfig$use_cache(ptr)
    }

    public func enable_quality_processing() -> Bool {
        __swift_bridge__$ExtractionConfig$enable_quality_processing(ptr)
    }

    public func ocr() -> Optional<OcrConfig> {
        { let val = __swift_bridge__$ExtractionConfig$ocr(ptr); if val != nil { return OcrConfig(ptr: val!) } else { return nil } }()
    }

    public func force_ocr() -> Bool {
        __swift_bridge__$ExtractionConfig$force_ocr(ptr)
    }

    public func force_ocr_pages() -> Optional<RustVec<UInt>> {
        { let val = __swift_bridge__$ExtractionConfig$force_ocr_pages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func disable_ocr() -> Bool {
        __swift_bridge__$ExtractionConfig$disable_ocr(ptr)
    }

    public func chunking() -> Optional<ChunkingConfig> {
        { let val = __swift_bridge__$ExtractionConfig$chunking(ptr); if val != nil { return ChunkingConfig(ptr: val!) } else { return nil } }()
    }

    public func content_filter() -> Optional<ContentFilterConfig> {
        { let val = __swift_bridge__$ExtractionConfig$content_filter(ptr); if val != nil { return ContentFilterConfig(ptr: val!) } else { return nil } }()
    }

    public func images() -> Optional<ImageExtractionConfig> {
        { let val = __swift_bridge__$ExtractionConfig$images(ptr); if val != nil { return ImageExtractionConfig(ptr: val!) } else { return nil } }()
    }

    public func pdf_options() -> Optional<PdfConfig> {
        { let val = __swift_bridge__$ExtractionConfig$pdf_options(ptr); if val != nil { return PdfConfig(ptr: val!) } else { return nil } }()
    }

    public func token_reduction() -> Optional<TokenReductionOptions> {
        { let val = __swift_bridge__$ExtractionConfig$token_reduction(ptr); if val != nil { return TokenReductionOptions(ptr: val!) } else { return nil } }()
    }

    public func language_detection() -> Optional<LanguageDetectionConfig> {
        { let val = __swift_bridge__$ExtractionConfig$language_detection(ptr); if val != nil { return LanguageDetectionConfig(ptr: val!) } else { return nil } }()
    }

    public func pages() -> Optional<PageConfig> {
        { let val = __swift_bridge__$ExtractionConfig$pages(ptr); if val != nil { return PageConfig(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> Optional<KeywordConfig> {
        { let val = __swift_bridge__$ExtractionConfig$keywords(ptr); if val != nil { return KeywordConfig(ptr: val!) } else { return nil } }()
    }

    public func postprocessor() -> Optional<PostProcessorConfig> {
        { let val = __swift_bridge__$ExtractionConfig$postprocessor(ptr); if val != nil { return PostProcessorConfig(ptr: val!) } else { return nil } }()
    }

    public func html_options() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionConfig$html_options(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func html_output() -> Optional<HtmlOutputConfig> {
        { let val = __swift_bridge__$ExtractionConfig$html_output(ptr); if val != nil { return HtmlOutputConfig(ptr: val!) } else { return nil } }()
    }

    public func extraction_timeout_secs() -> Optional<UInt64> {
        __swift_bridge__$ExtractionConfig$extraction_timeout_secs(ptr).intoSwiftRepr()
    }

    public func max_concurrent_extractions() -> Optional<UInt> {
        __swift_bridge__$ExtractionConfig$max_concurrent_extractions(ptr).intoSwiftRepr()
    }

    public func result_format() -> ResultFormat {
        ResultFormat(ptr: __swift_bridge__$ExtractionConfig$result_format(ptr))
    }

    public func security_limits() -> Optional<SecurityLimits> {
        { let val = __swift_bridge__$ExtractionConfig$security_limits(ptr); if val != nil { return SecurityLimits(ptr: val!) } else { return nil } }()
    }

    public func output_format() -> OutputFormat {
        OutputFormat(ptr: __swift_bridge__$ExtractionConfig$output_format(ptr))
    }

    public func layout() -> Optional<LayoutDetectionConfig> {
        { let val = __swift_bridge__$ExtractionConfig$layout(ptr); if val != nil { return LayoutDetectionConfig(ptr: val!) } else { return nil } }()
    }

    public func use_layout_for_markdown() -> Bool {
        __swift_bridge__$ExtractionConfig$use_layout_for_markdown(ptr)
    }

    public func include_document_structure() -> Bool {
        __swift_bridge__$ExtractionConfig$include_document_structure(ptr)
    }

    public func acceleration() -> Optional<AccelerationConfig> {
        { let val = __swift_bridge__$ExtractionConfig$acceleration(ptr); if val != nil { return AccelerationConfig(ptr: val!) } else { return nil } }()
    }

    public func cache_namespace() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionConfig$cache_namespace(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func cache_ttl_secs() -> Optional<UInt64> {
        __swift_bridge__$ExtractionConfig$cache_ttl_secs(ptr).intoSwiftRepr()
    }

    public func email() -> Optional<EmailConfig> {
        { let val = __swift_bridge__$ExtractionConfig$email(ptr); if val != nil { return EmailConfig(ptr: val!) } else { return nil } }()
    }

    public func concurrency() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionConfig$concurrency(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func max_archive_depth() -> UInt {
        __swift_bridge__$ExtractionConfig$max_archive_depth(ptr)
    }

    public func tree_sitter() -> Optional<TreeSitterConfig> {
        { let val = __swift_bridge__$ExtractionConfig$tree_sitter(ptr); if val != nil { return TreeSitterConfig(ptr: val!) } else { return nil } }()
    }

    public func structured_extraction() -> Optional<StructuredExtractionConfig> {
        { let val = __swift_bridge__$ExtractionConfig$structured_extraction(ptr); if val != nil { return StructuredExtractionConfig(ptr: val!) } else { return nil } }()
    }

    public func cancel_token() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionConfig$cancel_token(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension ExtractionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractionConfig) {
        __swift_bridge__$Vec_ExtractionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionConfigRef> {
        let pointer = __swift_bridge__$Vec_ExtractionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractionConfigRef> {
        UnsafePointer<ExtractionConfigRef>(OpaquePointer(__swift_bridge__$Vec_ExtractionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractionConfig$len(vecPtr)
    }
}


public class FileExtractionConfig: FileExtractionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$FileExtractionConfig$_free(ptr)
        }
    }
}
extension FileExtractionConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ enable_quality_processing: Optional<Bool>, _ ocr: Optional<OcrConfig>, _ force_ocr: Optional<Bool>, _ force_ocr_pages: Optional<RustVec<UInt>>, _ disable_ocr: Optional<Bool>, _ chunking: Optional<ChunkingConfig>, _ content_filter: Optional<ContentFilterConfig>, _ images: Optional<ImageExtractionConfig>, _ pdf_options: Optional<PdfConfig>, _ token_reduction: Optional<TokenReductionOptions>, _ language_detection: Optional<LanguageDetectionConfig>, _ pages: Optional<PageConfig>, _ keywords: Optional<KeywordConfig>, _ postprocessor: Optional<PostProcessorConfig>, _ html_options: Optional<GenericIntoRustString>, _ result_format: Optional<ResultFormat>, _ output_format: Optional<OutputFormat>, _ include_document_structure: Optional<Bool>, _ layout: Optional<LayoutDetectionConfig>, _ timeout_secs: Optional<UInt64>, _ tree_sitter: Optional<TreeSitterConfig>, _ structured_extraction: Optional<StructuredExtractionConfig>) {
        self.init(ptr: __swift_bridge__$FileExtractionConfig$new(enable_quality_processing.intoFfiRepr(), { if let val = ocr { val.isOwned = false; return val.ptr } else { return nil } }(), force_ocr.intoFfiRepr(), { if let val = force_ocr_pages { val.isOwned = false; return val.ptr } else { return nil } }(), disable_ocr.intoFfiRepr(), { if let val = chunking { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = content_filter { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = images { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pdf_options { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = token_reduction { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = language_detection { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = keywords { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = postprocessor { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(html_options) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = result_format { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = output_format { val.isOwned = false; return val.ptr } else { return nil } }(), include_document_structure.intoFfiRepr(), { if let val = layout { val.isOwned = false; return val.ptr } else { return nil } }(), timeout_secs.intoFfiRepr(), { if let val = tree_sitter { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = structured_extraction { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class FileExtractionConfigRefMut: FileExtractionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FileExtractionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FileExtractionConfigRef {
    public func enable_quality_processing() -> Optional<Bool> {
        __swift_bridge__$FileExtractionConfig$enable_quality_processing(ptr).intoSwiftRepr()
    }

    public func ocr() -> Optional<OcrConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$ocr(ptr); if val != nil { return OcrConfig(ptr: val!) } else { return nil } }()
    }

    public func force_ocr() -> Optional<Bool> {
        __swift_bridge__$FileExtractionConfig$force_ocr(ptr).intoSwiftRepr()
    }

    public func force_ocr_pages() -> Optional<RustVec<UInt>> {
        { let val = __swift_bridge__$FileExtractionConfig$force_ocr_pages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func disable_ocr() -> Optional<Bool> {
        __swift_bridge__$FileExtractionConfig$disable_ocr(ptr).intoSwiftRepr()
    }

    public func chunking() -> Optional<ChunkingConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$chunking(ptr); if val != nil { return ChunkingConfig(ptr: val!) } else { return nil } }()
    }

    public func content_filter() -> Optional<ContentFilterConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$content_filter(ptr); if val != nil { return ContentFilterConfig(ptr: val!) } else { return nil } }()
    }

    public func images() -> Optional<ImageExtractionConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$images(ptr); if val != nil { return ImageExtractionConfig(ptr: val!) } else { return nil } }()
    }

    public func pdf_options() -> Optional<PdfConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$pdf_options(ptr); if val != nil { return PdfConfig(ptr: val!) } else { return nil } }()
    }

    public func token_reduction() -> Optional<TokenReductionOptions> {
        { let val = __swift_bridge__$FileExtractionConfig$token_reduction(ptr); if val != nil { return TokenReductionOptions(ptr: val!) } else { return nil } }()
    }

    public func language_detection() -> Optional<LanguageDetectionConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$language_detection(ptr); if val != nil { return LanguageDetectionConfig(ptr: val!) } else { return nil } }()
    }

    public func pages() -> Optional<PageConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$pages(ptr); if val != nil { return PageConfig(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> Optional<KeywordConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$keywords(ptr); if val != nil { return KeywordConfig(ptr: val!) } else { return nil } }()
    }

    public func postprocessor() -> Optional<PostProcessorConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$postprocessor(ptr); if val != nil { return PostProcessorConfig(ptr: val!) } else { return nil } }()
    }

    public func html_options() -> Optional<RustString> {
        { let val = __swift_bridge__$FileExtractionConfig$html_options(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func result_format() -> Optional<ResultFormat> {
        { let val = __swift_bridge__$FileExtractionConfig$result_format(ptr); if val != nil { return ResultFormat(ptr: val!) } else { return nil } }()
    }

    public func output_format() -> Optional<OutputFormat> {
        { let val = __swift_bridge__$FileExtractionConfig$output_format(ptr); if val != nil { return OutputFormat(ptr: val!) } else { return nil } }()
    }

    public func include_document_structure() -> Optional<Bool> {
        __swift_bridge__$FileExtractionConfig$include_document_structure(ptr).intoSwiftRepr()
    }

    public func layout() -> Optional<LayoutDetectionConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$layout(ptr); if val != nil { return LayoutDetectionConfig(ptr: val!) } else { return nil } }()
    }

    public func timeout_secs() -> Optional<UInt64> {
        __swift_bridge__$FileExtractionConfig$timeout_secs(ptr).intoSwiftRepr()
    }

    public func tree_sitter() -> Optional<TreeSitterConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$tree_sitter(ptr); if val != nil { return TreeSitterConfig(ptr: val!) } else { return nil } }()
    }

    public func structured_extraction() -> Optional<StructuredExtractionConfig> {
        { let val = __swift_bridge__$FileExtractionConfig$structured_extraction(ptr); if val != nil { return StructuredExtractionConfig(ptr: val!) } else { return nil } }()
    }
}
extension FileExtractionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_FileExtractionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_FileExtractionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: FileExtractionConfig) {
        __swift_bridge__$Vec_FileExtractionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_FileExtractionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (FileExtractionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FileExtractionConfigRef> {
        let pointer = __swift_bridge__$Vec_FileExtractionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FileExtractionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FileExtractionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_FileExtractionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FileExtractionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FileExtractionConfigRef> {
        UnsafePointer<FileExtractionConfigRef>(OpaquePointer(__swift_bridge__$Vec_FileExtractionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_FileExtractionConfig$len(vecPtr)
    }
}


public class BatchBytesItem: BatchBytesItemRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$BatchBytesItem$_free(ptr)
        }
    }
}
public class BatchBytesItemRefMut: BatchBytesItemRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class BatchBytesItemRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension BatchBytesItemRef {
    public func content() -> RustVec<UInt8> {
        RustVec(ptr: __swift_bridge__$BatchBytesItem$content(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$BatchBytesItem$mime_type(ptr))
    }

    public func config() -> Optional<FileExtractionConfig> {
        { let val = __swift_bridge__$BatchBytesItem$config(ptr); if val != nil { return FileExtractionConfig(ptr: val!) } else { return nil } }()
    }
}
extension BatchBytesItem: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_BatchBytesItem$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_BatchBytesItem$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: BatchBytesItem) {
        __swift_bridge__$Vec_BatchBytesItem$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_BatchBytesItem$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (BatchBytesItem(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BatchBytesItemRef> {
        let pointer = __swift_bridge__$Vec_BatchBytesItem$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BatchBytesItemRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BatchBytesItemRefMut> {
        let pointer = __swift_bridge__$Vec_BatchBytesItem$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BatchBytesItemRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<BatchBytesItemRef> {
        UnsafePointer<BatchBytesItemRef>(OpaquePointer(__swift_bridge__$Vec_BatchBytesItem$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_BatchBytesItem$len(vecPtr)
    }
}


public class BatchFileItem: BatchFileItemRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$BatchFileItem$_free(ptr)
        }
    }
}
public class BatchFileItemRefMut: BatchFileItemRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class BatchFileItemRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension BatchFileItemRef {
    public func path() -> RustString {
        RustString(ptr: __swift_bridge__$BatchFileItem$path(ptr))
    }

    public func config() -> Optional<FileExtractionConfig> {
        { let val = __swift_bridge__$BatchFileItem$config(ptr); if val != nil { return FileExtractionConfig(ptr: val!) } else { return nil } }()
    }
}
extension BatchFileItem: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_BatchFileItem$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_BatchFileItem$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: BatchFileItem) {
        __swift_bridge__$Vec_BatchFileItem$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_BatchFileItem$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (BatchFileItem(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BatchFileItemRef> {
        let pointer = __swift_bridge__$Vec_BatchFileItem$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BatchFileItemRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BatchFileItemRefMut> {
        let pointer = __swift_bridge__$Vec_BatchFileItem$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BatchFileItemRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<BatchFileItemRef> {
        UnsafePointer<BatchFileItemRef>(OpaquePointer(__swift_bridge__$Vec_BatchFileItem$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_BatchFileItem$len(vecPtr)
    }
}


public class ImageExtractionConfig: ImageExtractionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageExtractionConfig$_free(ptr)
        }
    }
}
extension ImageExtractionConfig {
    public convenience init(_ extract_images: Bool, _ target_dpi: Int32, _ max_image_dimension: Int32, _ inject_placeholders: Bool, _ auto_adjust_dpi: Bool, _ min_dpi: Int32, _ max_dpi: Int32, _ max_images_per_page: Optional<UInt32>, _ classify: Bool) {
        self.init(ptr: __swift_bridge__$ImageExtractionConfig$new(extract_images, target_dpi, max_image_dimension, inject_placeholders, auto_adjust_dpi, min_dpi, max_dpi, max_images_per_page.intoFfiRepr(), classify))
    }
}
public class ImageExtractionConfigRefMut: ImageExtractionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageExtractionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageExtractionConfigRef {
    public func extract_images() -> Bool {
        __swift_bridge__$ImageExtractionConfig$extract_images(ptr)
    }

    public func target_dpi() -> Int32 {
        __swift_bridge__$ImageExtractionConfig$target_dpi(ptr)
    }

    public func max_image_dimension() -> Int32 {
        __swift_bridge__$ImageExtractionConfig$max_image_dimension(ptr)
    }

    public func inject_placeholders() -> Bool {
        __swift_bridge__$ImageExtractionConfig$inject_placeholders(ptr)
    }

    public func auto_adjust_dpi() -> Bool {
        __swift_bridge__$ImageExtractionConfig$auto_adjust_dpi(ptr)
    }

    public func min_dpi() -> Int32 {
        __swift_bridge__$ImageExtractionConfig$min_dpi(ptr)
    }

    public func max_dpi() -> Int32 {
        __swift_bridge__$ImageExtractionConfig$max_dpi(ptr)
    }

    public func max_images_per_page() -> Optional<UInt32> {
        __swift_bridge__$ImageExtractionConfig$max_images_per_page(ptr).intoSwiftRepr()
    }

    public func classify() -> Bool {
        __swift_bridge__$ImageExtractionConfig$classify(ptr)
    }
}
extension ImageExtractionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageExtractionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageExtractionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageExtractionConfig) {
        __swift_bridge__$Vec_ImageExtractionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageExtractionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageExtractionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageExtractionConfigRef> {
        let pointer = __swift_bridge__$Vec_ImageExtractionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageExtractionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageExtractionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ImageExtractionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageExtractionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageExtractionConfigRef> {
        UnsafePointer<ImageExtractionConfigRef>(OpaquePointer(__swift_bridge__$Vec_ImageExtractionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageExtractionConfig$len(vecPtr)
    }
}


public class TokenReductionOptions: TokenReductionOptionsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TokenReductionOptions$_free(ptr)
        }
    }
}
extension TokenReductionOptions {
    public convenience init<GenericIntoRustString: IntoRustString>(_ mode: GenericIntoRustString, _ preserve_important_words: Bool) {
        self.init(ptr: __swift_bridge__$TokenReductionOptions$new({ let rustString = mode.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), preserve_important_words))
    }
}
public class TokenReductionOptionsRefMut: TokenReductionOptionsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TokenReductionOptionsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TokenReductionOptionsRef {
    public func mode() -> RustString {
        RustString(ptr: __swift_bridge__$TokenReductionOptions$mode(ptr))
    }

    public func preserve_important_words() -> Bool {
        __swift_bridge__$TokenReductionOptions$preserve_important_words(ptr)
    }
}
extension TokenReductionOptions: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TokenReductionOptions$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TokenReductionOptions$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TokenReductionOptions) {
        __swift_bridge__$Vec_TokenReductionOptions$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TokenReductionOptions$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TokenReductionOptions(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TokenReductionOptionsRef> {
        let pointer = __swift_bridge__$Vec_TokenReductionOptions$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TokenReductionOptionsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TokenReductionOptionsRefMut> {
        let pointer = __swift_bridge__$Vec_TokenReductionOptions$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TokenReductionOptionsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TokenReductionOptionsRef> {
        UnsafePointer<TokenReductionOptionsRef>(OpaquePointer(__swift_bridge__$Vec_TokenReductionOptions$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TokenReductionOptions$len(vecPtr)
    }
}


public class LanguageDetectionConfig: LanguageDetectionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LanguageDetectionConfig$_free(ptr)
        }
    }
}
extension LanguageDetectionConfig {
    public convenience init(_ enabled: Bool, _ min_confidence: Double, _ detect_multiple: Bool) {
        self.init(ptr: __swift_bridge__$LanguageDetectionConfig$new(enabled, min_confidence, detect_multiple))
    }
}
public class LanguageDetectionConfigRefMut: LanguageDetectionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LanguageDetectionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LanguageDetectionConfigRef {
    public func enabled() -> Bool {
        __swift_bridge__$LanguageDetectionConfig$enabled(ptr)
    }

    public func min_confidence() -> Double {
        __swift_bridge__$LanguageDetectionConfig$min_confidence(ptr)
    }

    public func detect_multiple() -> Bool {
        __swift_bridge__$LanguageDetectionConfig$detect_multiple(ptr)
    }
}
extension LanguageDetectionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LanguageDetectionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LanguageDetectionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LanguageDetectionConfig) {
        __swift_bridge__$Vec_LanguageDetectionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LanguageDetectionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LanguageDetectionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LanguageDetectionConfigRef> {
        let pointer = __swift_bridge__$Vec_LanguageDetectionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LanguageDetectionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LanguageDetectionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_LanguageDetectionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LanguageDetectionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LanguageDetectionConfigRef> {
        UnsafePointer<LanguageDetectionConfigRef>(OpaquePointer(__swift_bridge__$Vec_LanguageDetectionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LanguageDetectionConfig$len(vecPtr)
    }
}


public class HtmlOutputConfig: HtmlOutputConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HtmlOutputConfig$_free(ptr)
        }
    }
}
extension HtmlOutputConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ css: Optional<GenericIntoRustString>, _ css_file: Optional<GenericIntoRustString>, _ theme: HtmlTheme, _ class_prefix: GenericIntoRustString, _ embed_css: Bool) {
        self.init(ptr: __swift_bridge__$HtmlOutputConfig$new({ if let rustString = optionalStringIntoRustString(css) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(css_file) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), {theme.isOwned = false; return theme.ptr;}(), { let rustString = class_prefix.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), embed_css))
    }
}
public class HtmlOutputConfigRefMut: HtmlOutputConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HtmlOutputConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HtmlOutputConfigRef {
    public func css() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlOutputConfig$css(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func css_file() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlOutputConfig$css_file(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func theme() -> HtmlTheme {
        HtmlTheme(ptr: __swift_bridge__$HtmlOutputConfig$theme(ptr))
    }

    public func class_prefix() -> RustString {
        RustString(ptr: __swift_bridge__$HtmlOutputConfig$class_prefix(ptr))
    }

    public func embed_css() -> Bool {
        __swift_bridge__$HtmlOutputConfig$embed_css(ptr)
    }
}
extension HtmlOutputConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HtmlOutputConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HtmlOutputConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HtmlOutputConfig) {
        __swift_bridge__$Vec_HtmlOutputConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HtmlOutputConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HtmlOutputConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlOutputConfigRef> {
        let pointer = __swift_bridge__$Vec_HtmlOutputConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlOutputConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlOutputConfigRefMut> {
        let pointer = __swift_bridge__$Vec_HtmlOutputConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlOutputConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HtmlOutputConfigRef> {
        UnsafePointer<HtmlOutputConfigRef>(OpaquePointer(__swift_bridge__$Vec_HtmlOutputConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HtmlOutputConfig$len(vecPtr)
    }
}


public class LayoutDetectionConfig: LayoutDetectionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LayoutDetectionConfig$_free(ptr)
        }
    }
}
extension LayoutDetectionConfig {
    public convenience init(_ confidence_threshold: Optional<Float>, _ apply_heuristics: Bool, _ table_model: TableModel, _ acceleration: Optional<AccelerationConfig>) {
        self.init(ptr: __swift_bridge__$LayoutDetectionConfig$new(confidence_threshold.intoFfiRepr(), apply_heuristics, {table_model.isOwned = false; return table_model.ptr;}(), { if let val = acceleration { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class LayoutDetectionConfigRefMut: LayoutDetectionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LayoutDetectionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LayoutDetectionConfigRef {
    public func confidence_threshold() -> Optional<Float> {
        __swift_bridge__$LayoutDetectionConfig$confidence_threshold(ptr).intoSwiftRepr()
    }

    public func apply_heuristics() -> Bool {
        __swift_bridge__$LayoutDetectionConfig$apply_heuristics(ptr)
    }

    public func table_model() -> TableModel {
        TableModel(ptr: __swift_bridge__$LayoutDetectionConfig$table_model(ptr))
    }

    public func acceleration() -> Optional<AccelerationConfig> {
        { let val = __swift_bridge__$LayoutDetectionConfig$acceleration(ptr); if val != nil { return AccelerationConfig(ptr: val!) } else { return nil } }()
    }
}
extension LayoutDetectionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LayoutDetectionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LayoutDetectionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LayoutDetectionConfig) {
        __swift_bridge__$Vec_LayoutDetectionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LayoutDetectionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LayoutDetectionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutDetectionConfigRef> {
        let pointer = __swift_bridge__$Vec_LayoutDetectionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutDetectionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutDetectionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_LayoutDetectionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutDetectionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LayoutDetectionConfigRef> {
        UnsafePointer<LayoutDetectionConfigRef>(OpaquePointer(__swift_bridge__$Vec_LayoutDetectionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LayoutDetectionConfig$len(vecPtr)
    }
}


public class LlmConfig: LlmConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LlmConfig$_free(ptr)
        }
    }
}
extension LlmConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ model: GenericIntoRustString, _ api_key: Optional<GenericIntoRustString>, _ base_url: Optional<GenericIntoRustString>, _ timeout_secs: Optional<UInt64>, _ max_retries: Optional<UInt32>, _ temperature: Optional<Double>, _ max_tokens: Optional<UInt64>) {
        self.init(ptr: __swift_bridge__$LlmConfig$new({ let rustString = model.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let rustString = optionalStringIntoRustString(api_key) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(base_url) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), timeout_secs.intoFfiRepr(), max_retries.intoFfiRepr(), temperature.intoFfiRepr(), max_tokens.intoFfiRepr()))
    }
}
public class LlmConfigRefMut: LlmConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LlmConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LlmConfigRef {
    public func model() -> RustString {
        RustString(ptr: __swift_bridge__$LlmConfig$model(ptr))
    }

    public func api_key() -> Optional<RustString> {
        { let val = __swift_bridge__$LlmConfig$api_key(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func base_url() -> Optional<RustString> {
        { let val = __swift_bridge__$LlmConfig$base_url(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func timeout_secs() -> Optional<UInt64> {
        __swift_bridge__$LlmConfig$timeout_secs(ptr).intoSwiftRepr()
    }

    public func max_retries() -> Optional<UInt32> {
        __swift_bridge__$LlmConfig$max_retries(ptr).intoSwiftRepr()
    }

    public func temperature() -> Optional<Double> {
        __swift_bridge__$LlmConfig$temperature(ptr).intoSwiftRepr()
    }

    public func max_tokens() -> Optional<UInt64> {
        __swift_bridge__$LlmConfig$max_tokens(ptr).intoSwiftRepr()
    }
}
extension LlmConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LlmConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LlmConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LlmConfig) {
        __swift_bridge__$Vec_LlmConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LlmConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LlmConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LlmConfigRef> {
        let pointer = __swift_bridge__$Vec_LlmConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LlmConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LlmConfigRefMut> {
        let pointer = __swift_bridge__$Vec_LlmConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LlmConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LlmConfigRef> {
        UnsafePointer<LlmConfigRef>(OpaquePointer(__swift_bridge__$Vec_LlmConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LlmConfig$len(vecPtr)
    }
}


public class StructuredExtractionConfig: StructuredExtractionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StructuredExtractionConfig$_free(ptr)
        }
    }
}
public class StructuredExtractionConfigRefMut: StructuredExtractionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StructuredExtractionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StructuredExtractionConfigRef {
    public func schema() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredExtractionConfig$schema(ptr))
    }

    public func schema_name() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredExtractionConfig$schema_name(ptr))
    }

    public func schema_description() -> Optional<RustString> {
        { let val = __swift_bridge__$StructuredExtractionConfig$schema_description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func strict() -> Bool {
        __swift_bridge__$StructuredExtractionConfig$strict(ptr)
    }

    public func prompt() -> Optional<RustString> {
        { let val = __swift_bridge__$StructuredExtractionConfig$prompt(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func llm() -> LlmConfig {
        LlmConfig(ptr: __swift_bridge__$StructuredExtractionConfig$llm(ptr))
    }
}
extension StructuredExtractionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StructuredExtractionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StructuredExtractionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StructuredExtractionConfig) {
        __swift_bridge__$Vec_StructuredExtractionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StructuredExtractionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredExtractionConfigRef> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredExtractionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredExtractionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredExtractionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StructuredExtractionConfigRef> {
        UnsafePointer<StructuredExtractionConfigRef>(OpaquePointer(__swift_bridge__$Vec_StructuredExtractionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StructuredExtractionConfig$len(vecPtr)
    }
}


public class OcrQualityThresholds: OcrQualityThresholdsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrQualityThresholds$_free(ptr)
        }
    }
}
extension OcrQualityThresholds {
    public convenience init(_ min_total_non_whitespace: UInt, _ min_non_whitespace_per_page: Double, _ min_meaningful_word_len: UInt, _ min_meaningful_words: UInt, _ min_alnum_ratio: Double, _ min_garbage_chars: UInt, _ max_fragmented_word_ratio: Double, _ critical_fragmented_word_ratio: Double, _ min_avg_word_length: Double, _ min_words_for_avg_length_check: UInt, _ min_consecutive_repeat_ratio: Double, _ min_words_for_repeat_check: UInt, _ substantive_min_chars: UInt, _ non_text_min_chars: UInt, _ alnum_ws_ratio_threshold: Double, _ pipeline_min_quality: Double) {
        self.init(ptr: __swift_bridge__$OcrQualityThresholds$new(min_total_non_whitespace, min_non_whitespace_per_page, min_meaningful_word_len, min_meaningful_words, min_alnum_ratio, min_garbage_chars, max_fragmented_word_ratio, critical_fragmented_word_ratio, min_avg_word_length, min_words_for_avg_length_check, min_consecutive_repeat_ratio, min_words_for_repeat_check, substantive_min_chars, non_text_min_chars, alnum_ws_ratio_threshold, pipeline_min_quality))
    }
}
public class OcrQualityThresholdsRefMut: OcrQualityThresholdsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrQualityThresholdsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrQualityThresholdsRef {
    public func min_total_non_whitespace() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_total_non_whitespace(ptr)
    }

    public func min_non_whitespace_per_page() -> Double {
        __swift_bridge__$OcrQualityThresholds$min_non_whitespace_per_page(ptr)
    }

    public func min_meaningful_word_len() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_meaningful_word_len(ptr)
    }

    public func min_meaningful_words() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_meaningful_words(ptr)
    }

    public func min_alnum_ratio() -> Double {
        __swift_bridge__$OcrQualityThresholds$min_alnum_ratio(ptr)
    }

    public func min_garbage_chars() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_garbage_chars(ptr)
    }

    public func max_fragmented_word_ratio() -> Double {
        __swift_bridge__$OcrQualityThresholds$max_fragmented_word_ratio(ptr)
    }

    public func critical_fragmented_word_ratio() -> Double {
        __swift_bridge__$OcrQualityThresholds$critical_fragmented_word_ratio(ptr)
    }

    public func min_avg_word_length() -> Double {
        __swift_bridge__$OcrQualityThresholds$min_avg_word_length(ptr)
    }

    public func min_words_for_avg_length_check() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_words_for_avg_length_check(ptr)
    }

    public func min_consecutive_repeat_ratio() -> Double {
        __swift_bridge__$OcrQualityThresholds$min_consecutive_repeat_ratio(ptr)
    }

    public func min_words_for_repeat_check() -> UInt {
        __swift_bridge__$OcrQualityThresholds$min_words_for_repeat_check(ptr)
    }

    public func substantive_min_chars() -> UInt {
        __swift_bridge__$OcrQualityThresholds$substantive_min_chars(ptr)
    }

    public func non_text_min_chars() -> UInt {
        __swift_bridge__$OcrQualityThresholds$non_text_min_chars(ptr)
    }

    public func alnum_ws_ratio_threshold() -> Double {
        __swift_bridge__$OcrQualityThresholds$alnum_ws_ratio_threshold(ptr)
    }

    public func pipeline_min_quality() -> Double {
        __swift_bridge__$OcrQualityThresholds$pipeline_min_quality(ptr)
    }
}
extension OcrQualityThresholds: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrQualityThresholds$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrQualityThresholds$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrQualityThresholds) {
        __swift_bridge__$Vec_OcrQualityThresholds$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrQualityThresholds$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrQualityThresholds(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrQualityThresholdsRef> {
        let pointer = __swift_bridge__$Vec_OcrQualityThresholds$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrQualityThresholdsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrQualityThresholdsRefMut> {
        let pointer = __swift_bridge__$Vec_OcrQualityThresholds$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrQualityThresholdsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrQualityThresholdsRef> {
        UnsafePointer<OcrQualityThresholdsRef>(OpaquePointer(__swift_bridge__$Vec_OcrQualityThresholds$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrQualityThresholds$len(vecPtr)
    }
}


public class OcrPipelineStage: OcrPipelineStageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrPipelineStage$_free(ptr)
        }
    }
}
public class OcrPipelineStageRefMut: OcrPipelineStageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrPipelineStageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrPipelineStageRef {
    public func backend() -> RustString {
        RustString(ptr: __swift_bridge__$OcrPipelineStage$backend(ptr))
    }

    public func priority() -> UInt32 {
        __swift_bridge__$OcrPipelineStage$priority(ptr)
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$OcrPipelineStage$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func tesseract_config() -> Optional<TesseractConfig> {
        { let val = __swift_bridge__$OcrPipelineStage$tesseract_config(ptr); if val != nil { return TesseractConfig(ptr: val!) } else { return nil } }()
    }

    public func paddle_ocr_config() -> Optional<RustString> {
        { let val = __swift_bridge__$OcrPipelineStage$paddle_ocr_config(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func vlm_config() -> Optional<LlmConfig> {
        { let val = __swift_bridge__$OcrPipelineStage$vlm_config(ptr); if val != nil { return LlmConfig(ptr: val!) } else { return nil } }()
    }
}
extension OcrPipelineStage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrPipelineStage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrPipelineStage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrPipelineStage) {
        __swift_bridge__$Vec_OcrPipelineStage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrPipelineStage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrPipelineStage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrPipelineStageRef> {
        let pointer = __swift_bridge__$Vec_OcrPipelineStage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrPipelineStageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrPipelineStageRefMut> {
        let pointer = __swift_bridge__$Vec_OcrPipelineStage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrPipelineStageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrPipelineStageRef> {
        UnsafePointer<OcrPipelineStageRef>(OpaquePointer(__swift_bridge__$Vec_OcrPipelineStage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrPipelineStage$len(vecPtr)
    }
}


public class OcrPipelineConfig: OcrPipelineConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrPipelineConfig$_free(ptr)
        }
    }
}
public class OcrPipelineConfigRefMut: OcrPipelineConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrPipelineConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrPipelineConfigRef {
    public func stages() -> RustVec<OcrPipelineStage> {
        RustVec(ptr: __swift_bridge__$OcrPipelineConfig$stages(ptr))
    }

    public func quality_thresholds() -> OcrQualityThresholds {
        OcrQualityThresholds(ptr: __swift_bridge__$OcrPipelineConfig$quality_thresholds(ptr))
    }
}
extension OcrPipelineConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrPipelineConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrPipelineConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrPipelineConfig) {
        __swift_bridge__$Vec_OcrPipelineConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrPipelineConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrPipelineConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrPipelineConfigRef> {
        let pointer = __swift_bridge__$Vec_OcrPipelineConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrPipelineConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrPipelineConfigRefMut> {
        let pointer = __swift_bridge__$Vec_OcrPipelineConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrPipelineConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrPipelineConfigRef> {
        UnsafePointer<OcrPipelineConfigRef>(OpaquePointer(__swift_bridge__$Vec_OcrPipelineConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrPipelineConfig$len(vecPtr)
    }
}


public class OcrConfig: OcrConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrConfig$_free(ptr)
        }
    }
}
extension OcrConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ enabled: Bool, _ backend: GenericIntoRustString, _ language: GenericIntoRustString, _ tesseract_config: Optional<TesseractConfig>, _ output_format: Optional<OutputFormat>, _ paddle_ocr_config: Optional<GenericIntoRustString>, _ element_config: Optional<OcrElementConfig>, _ quality_thresholds: Optional<OcrQualityThresholds>, _ pipeline: Optional<OcrPipelineConfig>, _ auto_rotate: Bool, _ vlm_config: Optional<LlmConfig>, _ vlm_prompt: Optional<GenericIntoRustString>, _ acceleration: Optional<AccelerationConfig>) {
        self.init(ptr: __swift_bridge__$OcrConfig$new(enabled, { let rustString = backend.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = language.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let val = tesseract_config { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = output_format { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(paddle_ocr_config) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = element_config { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = quality_thresholds { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pipeline { val.isOwned = false; return val.ptr } else { return nil } }(), auto_rotate, { if let val = vlm_config { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(vlm_prompt) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = acceleration { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class OcrConfigRefMut: OcrConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrConfigRef {
    public func enabled() -> Bool {
        __swift_bridge__$OcrConfig$enabled(ptr)
    }

    public func backend() -> RustString {
        RustString(ptr: __swift_bridge__$OcrConfig$backend(ptr))
    }

    public func language() -> RustString {
        RustString(ptr: __swift_bridge__$OcrConfig$language(ptr))
    }

    public func tesseract_config() -> Optional<TesseractConfig> {
        { let val = __swift_bridge__$OcrConfig$tesseract_config(ptr); if val != nil { return TesseractConfig(ptr: val!) } else { return nil } }()
    }

    public func output_format() -> Optional<OutputFormat> {
        { let val = __swift_bridge__$OcrConfig$output_format(ptr); if val != nil { return OutputFormat(ptr: val!) } else { return nil } }()
    }

    public func paddle_ocr_config() -> Optional<RustString> {
        { let val = __swift_bridge__$OcrConfig$paddle_ocr_config(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func element_config() -> Optional<OcrElementConfig> {
        { let val = __swift_bridge__$OcrConfig$element_config(ptr); if val != nil { return OcrElementConfig(ptr: val!) } else { return nil } }()
    }

    public func quality_thresholds() -> Optional<OcrQualityThresholds> {
        { let val = __swift_bridge__$OcrConfig$quality_thresholds(ptr); if val != nil { return OcrQualityThresholds(ptr: val!) } else { return nil } }()
    }

    public func pipeline() -> Optional<OcrPipelineConfig> {
        { let val = __swift_bridge__$OcrConfig$pipeline(ptr); if val != nil { return OcrPipelineConfig(ptr: val!) } else { return nil } }()
    }

    public func auto_rotate() -> Bool {
        __swift_bridge__$OcrConfig$auto_rotate(ptr)
    }

    public func vlm_config() -> Optional<LlmConfig> {
        { let val = __swift_bridge__$OcrConfig$vlm_config(ptr); if val != nil { return LlmConfig(ptr: val!) } else { return nil } }()
    }

    public func vlm_prompt() -> Optional<RustString> {
        { let val = __swift_bridge__$OcrConfig$vlm_prompt(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func acceleration() -> Optional<AccelerationConfig> {
        { let val = __swift_bridge__$OcrConfig$acceleration(ptr); if val != nil { return AccelerationConfig(ptr: val!) } else { return nil } }()
    }
}
extension OcrConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrConfig) {
        __swift_bridge__$Vec_OcrConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrConfigRef> {
        let pointer = __swift_bridge__$Vec_OcrConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrConfigRefMut> {
        let pointer = __swift_bridge__$Vec_OcrConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrConfigRef> {
        UnsafePointer<OcrConfigRef>(OpaquePointer(__swift_bridge__$Vec_OcrConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrConfig$len(vecPtr)
    }
}


public class PageConfig: PageConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageConfig$_free(ptr)
        }
    }
}
extension PageConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ extract_pages: Bool, _ insert_page_markers: Bool, _ marker_format: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$PageConfig$new(extract_pages, insert_page_markers, { let rustString = marker_format.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class PageConfigRefMut: PageConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageConfigRef {
    public func extract_pages() -> Bool {
        __swift_bridge__$PageConfig$extract_pages(ptr)
    }

    public func insert_page_markers() -> Bool {
        __swift_bridge__$PageConfig$insert_page_markers(ptr)
    }

    public func marker_format() -> RustString {
        RustString(ptr: __swift_bridge__$PageConfig$marker_format(ptr))
    }
}
extension PageConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageConfig) {
        __swift_bridge__$Vec_PageConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageConfigRef> {
        let pointer = __swift_bridge__$Vec_PageConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageConfigRefMut> {
        let pointer = __swift_bridge__$Vec_PageConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageConfigRef> {
        UnsafePointer<PageConfigRef>(OpaquePointer(__swift_bridge__$Vec_PageConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageConfig$len(vecPtr)
    }
}


public class PdfConfig: PdfConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PdfConfig$_free(ptr)
        }
    }
}
extension PdfConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ extract_images: Bool, _ passwords: Optional<RustVec<GenericIntoRustString>>, _ extract_metadata: Bool, _ hierarchy: Optional<HierarchyConfig>, _ extract_annotations: Bool, _ top_margin_fraction: Optional<Float>, _ bottom_margin_fraction: Optional<Float>, _ allow_single_column_tables: Bool) {
        self.init(ptr: __swift_bridge__$PdfConfig$new(extract_images, { if let val = passwords { val.isOwned = false; return val.ptr } else { return nil } }(), extract_metadata, { if let val = hierarchy { val.isOwned = false; return val.ptr } else { return nil } }(), extract_annotations, top_margin_fraction.intoFfiRepr(), bottom_margin_fraction.intoFfiRepr(), allow_single_column_tables))
    }
}
public class PdfConfigRefMut: PdfConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PdfConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PdfConfigRef {
    public func extract_images() -> Bool {
        __swift_bridge__$PdfConfig$extract_images(ptr)
    }

    public func passwords() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$PdfConfig$passwords(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func extract_metadata() -> Bool {
        __swift_bridge__$PdfConfig$extract_metadata(ptr)
    }

    public func hierarchy() -> Optional<HierarchyConfig> {
        { let val = __swift_bridge__$PdfConfig$hierarchy(ptr); if val != nil { return HierarchyConfig(ptr: val!) } else { return nil } }()
    }

    public func extract_annotations() -> Bool {
        __swift_bridge__$PdfConfig$extract_annotations(ptr)
    }

    public func top_margin_fraction() -> Optional<Float> {
        __swift_bridge__$PdfConfig$top_margin_fraction(ptr).intoSwiftRepr()
    }

    public func bottom_margin_fraction() -> Optional<Float> {
        __swift_bridge__$PdfConfig$bottom_margin_fraction(ptr).intoSwiftRepr()
    }

    public func allow_single_column_tables() -> Bool {
        __swift_bridge__$PdfConfig$allow_single_column_tables(ptr)
    }
}
extension PdfConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PdfConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PdfConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PdfConfig) {
        __swift_bridge__$Vec_PdfConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PdfConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PdfConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfConfigRef> {
        let pointer = __swift_bridge__$Vec_PdfConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfConfigRefMut> {
        let pointer = __swift_bridge__$Vec_PdfConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PdfConfigRef> {
        UnsafePointer<PdfConfigRef>(OpaquePointer(__swift_bridge__$Vec_PdfConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PdfConfig$len(vecPtr)
    }
}


public class HierarchyConfig: HierarchyConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HierarchyConfig$_free(ptr)
        }
    }
}
extension HierarchyConfig {
    public convenience init(_ enabled: Bool, _ k_clusters: UInt, _ include_bbox: Bool, _ ocr_coverage_threshold: Optional<Float>) {
        self.init(ptr: __swift_bridge__$HierarchyConfig$new(enabled, k_clusters, include_bbox, ocr_coverage_threshold.intoFfiRepr()))
    }
}
public class HierarchyConfigRefMut: HierarchyConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HierarchyConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HierarchyConfigRef {
    public func enabled() -> Bool {
        __swift_bridge__$HierarchyConfig$enabled(ptr)
    }

    public func k_clusters() -> UInt {
        __swift_bridge__$HierarchyConfig$k_clusters(ptr)
    }

    public func include_bbox() -> Bool {
        __swift_bridge__$HierarchyConfig$include_bbox(ptr)
    }

    public func ocr_coverage_threshold() -> Optional<Float> {
        __swift_bridge__$HierarchyConfig$ocr_coverage_threshold(ptr).intoSwiftRepr()
    }
}
extension HierarchyConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HierarchyConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HierarchyConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HierarchyConfig) {
        __swift_bridge__$Vec_HierarchyConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HierarchyConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HierarchyConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HierarchyConfigRef> {
        let pointer = __swift_bridge__$Vec_HierarchyConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HierarchyConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HierarchyConfigRefMut> {
        let pointer = __swift_bridge__$Vec_HierarchyConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HierarchyConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HierarchyConfigRef> {
        UnsafePointer<HierarchyConfigRef>(OpaquePointer(__swift_bridge__$Vec_HierarchyConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HierarchyConfig$len(vecPtr)
    }
}


public class PostProcessorConfig: PostProcessorConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PostProcessorConfig$_free(ptr)
        }
    }
}
extension PostProcessorConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ enabled: Bool, _ enabled_processors: Optional<RustVec<GenericIntoRustString>>, _ disabled_processors: Optional<RustVec<GenericIntoRustString>>, _ enabled_set: Optional<GenericIntoRustString>, _ disabled_set: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$PostProcessorConfig$new(enabled, { if let val = enabled_processors { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = disabled_processors { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(enabled_set) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(disabled_set) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class PostProcessorConfigRefMut: PostProcessorConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PostProcessorConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PostProcessorConfigRef {
    public func enabled() -> Bool {
        __swift_bridge__$PostProcessorConfig$enabled(ptr)
    }

    public func enabled_processors() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$PostProcessorConfig$enabled_processors(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func disabled_processors() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$PostProcessorConfig$disabled_processors(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func enabled_set() -> Optional<RustString> {
        { let val = __swift_bridge__$PostProcessorConfig$enabled_set(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func disabled_set() -> Optional<RustString> {
        { let val = __swift_bridge__$PostProcessorConfig$disabled_set(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension PostProcessorConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PostProcessorConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PostProcessorConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PostProcessorConfig) {
        __swift_bridge__$Vec_PostProcessorConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PostProcessorConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PostProcessorConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PostProcessorConfigRef> {
        let pointer = __swift_bridge__$Vec_PostProcessorConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PostProcessorConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PostProcessorConfigRefMut> {
        let pointer = __swift_bridge__$Vec_PostProcessorConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PostProcessorConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PostProcessorConfigRef> {
        UnsafePointer<PostProcessorConfigRef>(OpaquePointer(__swift_bridge__$Vec_PostProcessorConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PostProcessorConfig$len(vecPtr)
    }
}


public class ChunkingConfig: ChunkingConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkingConfig$_free(ptr)
        }
    }
}
extension ChunkingConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ max_characters: UInt, _ overlap: UInt, _ trim: Bool, _ chunker_type: ChunkerType, _ embedding: Optional<EmbeddingConfig>, _ preset: Optional<GenericIntoRustString>, _ sizing: ChunkSizing, _ prepend_heading_context: Bool, _ topic_threshold: Optional<Float>) {
        self.init(ptr: __swift_bridge__$ChunkingConfig$new(max_characters, overlap, trim, {chunker_type.isOwned = false; return chunker_type.ptr;}(), { if let val = embedding { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(preset) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), {sizing.isOwned = false; return sizing.ptr;}(), prepend_heading_context, topic_threshold.intoFfiRepr()))
    }
}
public class ChunkingConfigRefMut: ChunkingConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkingConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkingConfigRef {
    public func max_characters() -> UInt {
        __swift_bridge__$ChunkingConfig$max_characters(ptr)
    }

    public func overlap() -> UInt {
        __swift_bridge__$ChunkingConfig$overlap(ptr)
    }

    public func trim() -> Bool {
        __swift_bridge__$ChunkingConfig$trim(ptr)
    }

    public func chunker_type() -> ChunkerType {
        ChunkerType(ptr: __swift_bridge__$ChunkingConfig$chunker_type(ptr))
    }

    public func embedding() -> Optional<EmbeddingConfig> {
        { let val = __swift_bridge__$ChunkingConfig$embedding(ptr); if val != nil { return EmbeddingConfig(ptr: val!) } else { return nil } }()
    }

    public func preset() -> Optional<RustString> {
        { let val = __swift_bridge__$ChunkingConfig$preset(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func sizing() -> ChunkSizing {
        ChunkSizing(ptr: __swift_bridge__$ChunkingConfig$sizing(ptr))
    }

    public func prepend_heading_context() -> Bool {
        __swift_bridge__$ChunkingConfig$prepend_heading_context(ptr)
    }

    public func topic_threshold() -> Optional<Float> {
        __swift_bridge__$ChunkingConfig$topic_threshold(ptr).intoSwiftRepr()
    }
}
extension ChunkingConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkingConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkingConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkingConfig) {
        __swift_bridge__$Vec_ChunkingConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkingConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkingConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkingConfigRef> {
        let pointer = __swift_bridge__$Vec_ChunkingConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkingConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkingConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkingConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkingConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkingConfigRef> {
        UnsafePointer<ChunkingConfigRef>(OpaquePointer(__swift_bridge__$Vec_ChunkingConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkingConfig$len(vecPtr)
    }
}


public class EmbeddingConfig: EmbeddingConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbeddingConfig$_free(ptr)
        }
    }
}
extension EmbeddingConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ model: EmbeddingModelType, _ normalize: Bool, _ batch_size: UInt, _ show_download_progress: Bool, _ cache_dir: Optional<GenericIntoRustString>, _ acceleration: Optional<AccelerationConfig>, _ max_embed_duration_secs: Optional<UInt64>) {
        self.init(ptr: __swift_bridge__$EmbeddingConfig$new({model.isOwned = false; return model.ptr;}(), normalize, batch_size, show_download_progress, { if let rustString = optionalStringIntoRustString(cache_dir) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = acceleration { val.isOwned = false; return val.ptr } else { return nil } }(), max_embed_duration_secs.intoFfiRepr()))
    }
}
public class EmbeddingConfigRefMut: EmbeddingConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbeddingConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbeddingConfigRef {
    public func model() -> EmbeddingModelType {
        EmbeddingModelType(ptr: __swift_bridge__$EmbeddingConfig$model(ptr))
    }

    public func normalize() -> Bool {
        __swift_bridge__$EmbeddingConfig$normalize(ptr)
    }

    public func batch_size() -> UInt {
        __swift_bridge__$EmbeddingConfig$batch_size(ptr)
    }

    public func show_download_progress() -> Bool {
        __swift_bridge__$EmbeddingConfig$show_download_progress(ptr)
    }

    public func cache_dir() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbeddingConfig$cache_dir(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func acceleration() -> Optional<AccelerationConfig> {
        { let val = __swift_bridge__$EmbeddingConfig$acceleration(ptr); if val != nil { return AccelerationConfig(ptr: val!) } else { return nil } }()
    }

    public func max_embed_duration_secs() -> Optional<UInt64> {
        __swift_bridge__$EmbeddingConfig$max_embed_duration_secs(ptr).intoSwiftRepr()
    }
}
extension EmbeddingConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbeddingConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbeddingConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbeddingConfig) {
        __swift_bridge__$Vec_EmbeddingConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbeddingConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbeddingConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingConfigRef> {
        let pointer = __swift_bridge__$Vec_EmbeddingConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingConfigRefMut> {
        let pointer = __swift_bridge__$Vec_EmbeddingConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbeddingConfigRef> {
        UnsafePointer<EmbeddingConfigRef>(OpaquePointer(__swift_bridge__$Vec_EmbeddingConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbeddingConfig$len(vecPtr)
    }
}


public class TreeSitterConfig: TreeSitterConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TreeSitterConfig$_free(ptr)
        }
    }
}
extension TreeSitterConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ enabled: Bool, _ cache_dir: Optional<GenericIntoRustString>, _ languages: Optional<RustVec<GenericIntoRustString>>, _ groups: Optional<RustVec<GenericIntoRustString>>, _ process: TreeSitterProcessConfig) {
        self.init(ptr: __swift_bridge__$TreeSitterConfig$new(enabled, { if let rustString = optionalStringIntoRustString(cache_dir) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = languages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = groups { val.isOwned = false; return val.ptr } else { return nil } }(), {process.isOwned = false; return process.ptr;}()))
    }
}
public class TreeSitterConfigRefMut: TreeSitterConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TreeSitterConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TreeSitterConfigRef {
    public func enabled() -> Bool {
        __swift_bridge__$TreeSitterConfig$enabled(ptr)
    }

    public func cache_dir() -> Optional<RustString> {
        { let val = __swift_bridge__$TreeSitterConfig$cache_dir(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func languages() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TreeSitterConfig$languages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func groups() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TreeSitterConfig$groups(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func process() -> TreeSitterProcessConfig {
        TreeSitterProcessConfig(ptr: __swift_bridge__$TreeSitterConfig$process(ptr))
    }
}
extension TreeSitterConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TreeSitterConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TreeSitterConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TreeSitterConfig) {
        __swift_bridge__$Vec_TreeSitterConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TreeSitterConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TreeSitterConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TreeSitterConfigRef> {
        let pointer = __swift_bridge__$Vec_TreeSitterConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TreeSitterConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TreeSitterConfigRefMut> {
        let pointer = __swift_bridge__$Vec_TreeSitterConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TreeSitterConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TreeSitterConfigRef> {
        UnsafePointer<TreeSitterConfigRef>(OpaquePointer(__swift_bridge__$Vec_TreeSitterConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TreeSitterConfig$len(vecPtr)
    }
}


public class TreeSitterProcessConfig: TreeSitterProcessConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TreeSitterProcessConfig$_free(ptr)
        }
    }
}
extension TreeSitterProcessConfig {
    public convenience init(_ structure: Bool, _ imports: Bool, _ exports: Bool, _ comments: Bool, _ docstrings: Bool, _ symbols: Bool, _ diagnostics: Bool, _ chunk_max_size: Optional<UInt>, _ content_mode: CodeContentMode) {
        self.init(ptr: __swift_bridge__$TreeSitterProcessConfig$new(structure, imports, exports, comments, docstrings, symbols, diagnostics, chunk_max_size.intoFfiRepr(), {content_mode.isOwned = false; return content_mode.ptr;}()))
    }
}
public class TreeSitterProcessConfigRefMut: TreeSitterProcessConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TreeSitterProcessConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TreeSitterProcessConfigRef {
    public func structure() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$structure(ptr)
    }

    public func imports() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$imports(ptr)
    }

    public func exports() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$exports(ptr)
    }

    public func comments() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$comments(ptr)
    }

    public func docstrings() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$docstrings(ptr)
    }

    public func symbols() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$symbols(ptr)
    }

    public func diagnostics() -> Bool {
        __swift_bridge__$TreeSitterProcessConfig$diagnostics(ptr)
    }

    public func chunk_max_size() -> Optional<UInt> {
        __swift_bridge__$TreeSitterProcessConfig$chunk_max_size(ptr).intoSwiftRepr()
    }

    public func content_mode() -> CodeContentMode {
        CodeContentMode(ptr: __swift_bridge__$TreeSitterProcessConfig$content_mode(ptr))
    }
}
extension TreeSitterProcessConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TreeSitterProcessConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TreeSitterProcessConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TreeSitterProcessConfig) {
        __swift_bridge__$Vec_TreeSitterProcessConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TreeSitterProcessConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TreeSitterProcessConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TreeSitterProcessConfigRef> {
        let pointer = __swift_bridge__$Vec_TreeSitterProcessConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TreeSitterProcessConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TreeSitterProcessConfigRefMut> {
        let pointer = __swift_bridge__$Vec_TreeSitterProcessConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TreeSitterProcessConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TreeSitterProcessConfigRef> {
        UnsafePointer<TreeSitterProcessConfigRef>(OpaquePointer(__swift_bridge__$Vec_TreeSitterProcessConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TreeSitterProcessConfig$len(vecPtr)
    }
}


public class SupportedFormat: SupportedFormatRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$SupportedFormat$_free(ptr)
        }
    }
}
public class SupportedFormatRefMut: SupportedFormatRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class SupportedFormatRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension SupportedFormatRef {
    public func extension_() -> RustString {
        RustString(ptr: __swift_bridge__$SupportedFormat$extension_(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$SupportedFormat$mime_type(ptr))
    }
}
extension SupportedFormat: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_SupportedFormat$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_SupportedFormat$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: SupportedFormat) {
        __swift_bridge__$Vec_SupportedFormat$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_SupportedFormat$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (SupportedFormat(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SupportedFormatRef> {
        let pointer = __swift_bridge__$Vec_SupportedFormat$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return SupportedFormatRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SupportedFormatRefMut> {
        let pointer = __swift_bridge__$Vec_SupportedFormat$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return SupportedFormatRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<SupportedFormatRef> {
        UnsafePointer<SupportedFormatRef>(OpaquePointer(__swift_bridge__$Vec_SupportedFormat$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_SupportedFormat$len(vecPtr)
    }
}


public class ServerConfig: ServerConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ServerConfig$_free(ptr)
        }
    }
}
extension ServerConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ host: GenericIntoRustString, _ port: UInt16, _ cors_origins: RustVec<GenericIntoRustString>, _ max_request_body_bytes: UInt, _ max_multipart_field_bytes: UInt) {
        self.init(ptr: __swift_bridge__$ServerConfig$new({ let rustString = host.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), port, { let val = cors_origins; val.isOwned = false; return val.ptr }(), max_request_body_bytes, max_multipart_field_bytes))
    }
}
public class ServerConfigRefMut: ServerConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ServerConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ServerConfigRef {
    public func host() -> RustString {
        RustString(ptr: __swift_bridge__$ServerConfig$host(ptr))
    }

    public func port() -> UInt16 {
        __swift_bridge__$ServerConfig$port(ptr)
    }

    public func cors_origins() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$ServerConfig$cors_origins(ptr))
    }

    public func max_request_body_bytes() -> UInt {
        __swift_bridge__$ServerConfig$max_request_body_bytes(ptr)
    }

    public func max_multipart_field_bytes() -> UInt {
        __swift_bridge__$ServerConfig$max_multipart_field_bytes(ptr)
    }
}
extension ServerConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ServerConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ServerConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ServerConfig) {
        __swift_bridge__$Vec_ServerConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ServerConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ServerConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ServerConfigRef> {
        let pointer = __swift_bridge__$Vec_ServerConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ServerConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ServerConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ServerConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ServerConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ServerConfigRef> {
        UnsafePointer<ServerConfigRef>(OpaquePointer(__swift_bridge__$Vec_ServerConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ServerConfig$len(vecPtr)
    }
}


public class StructuredDataResult: StructuredDataResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StructuredDataResult$_free(ptr)
        }
    }
}
public class StructuredDataResultRefMut: StructuredDataResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StructuredDataResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StructuredDataResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredDataResult$content(ptr))
    }

    public func format() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredDataResult$format(ptr))
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredDataResult$metadata(ptr))
    }

    public func text_fields() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$StructuredDataResult$text_fields(ptr))
    }
}
extension StructuredDataResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StructuredDataResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StructuredDataResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StructuredDataResult) {
        __swift_bridge__$Vec_StructuredDataResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StructuredDataResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StructuredDataResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataResultRef> {
        let pointer = __swift_bridge__$Vec_StructuredDataResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataResultRefMut> {
        let pointer = __swift_bridge__$Vec_StructuredDataResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StructuredDataResultRef> {
        UnsafePointer<StructuredDataResultRef>(OpaquePointer(__swift_bridge__$Vec_StructuredDataResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StructuredDataResult$len(vecPtr)
    }
}


public class CharShape: CharShapeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CharShape$_free(ptr)
        }
    }
}
extension CharShape {
    public convenience init(_ bold: Bool, _ italic: Bool, _ underline: Bool) {
        self.init(ptr: __swift_bridge__$CharShape$new(bold, italic, underline))
    }
}
public class CharShapeRefMut: CharShapeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CharShapeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CharShapeRef {
    public func bold() -> Bool {
        __swift_bridge__$CharShape$bold(ptr)
    }

    public func italic() -> Bool {
        __swift_bridge__$CharShape$italic(ptr)
    }

    public func underline() -> Bool {
        __swift_bridge__$CharShape$underline(ptr)
    }
}
extension CharShape: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CharShape$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CharShape$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CharShape) {
        __swift_bridge__$Vec_CharShape$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CharShape$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CharShape(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CharShapeRef> {
        let pointer = __swift_bridge__$Vec_CharShape$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CharShapeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CharShapeRefMut> {
        let pointer = __swift_bridge__$Vec_CharShape$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CharShapeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CharShapeRef> {
        UnsafePointer<CharShapeRef>(OpaquePointer(__swift_bridge__$Vec_CharShape$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CharShape$len(vecPtr)
    }
}


public class HwpImage: HwpImageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HwpImage$_free(ptr)
        }
    }
}
extension HwpImage {
    public convenience init<GenericIntoRustString: IntoRustString>(_ name: GenericIntoRustString, _ data: RustVec<UInt8>) {
        self.init(ptr: __swift_bridge__$HwpImage$new({ let rustString = name.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = data; val.isOwned = false; return val.ptr }()))
    }
}
public class HwpImageRefMut: HwpImageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HwpImageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HwpImageRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$HwpImage$name(ptr))
    }

    public func data() -> RustVec<UInt8> {
        RustVec(ptr: __swift_bridge__$HwpImage$data(ptr))
    }
}
extension HwpImage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HwpImage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HwpImage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HwpImage) {
        __swift_bridge__$Vec_HwpImage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HwpImage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HwpImage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HwpImageRef> {
        let pointer = __swift_bridge__$Vec_HwpImage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HwpImageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HwpImageRefMut> {
        let pointer = __swift_bridge__$Vec_HwpImage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HwpImageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HwpImageRef> {
        UnsafePointer<HwpImageRef>(OpaquePointer(__swift_bridge__$Vec_HwpImage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HwpImage$len(vecPtr)
    }
}


public class StreamReader: StreamReaderRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StreamReader$_free(ptr)
        }
    }
}
public class StreamReaderRefMut: StreamReaderRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StreamReaderRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StreamReader: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StreamReader$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StreamReader$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StreamReader) {
        __swift_bridge__$Vec_StreamReader$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StreamReader$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StreamReader(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StreamReaderRef> {
        let pointer = __swift_bridge__$Vec_StreamReader$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StreamReaderRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StreamReaderRefMut> {
        let pointer = __swift_bridge__$Vec_StreamReader$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StreamReaderRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StreamReaderRef> {
        UnsafePointer<StreamReaderRef>(OpaquePointer(__swift_bridge__$Vec_StreamReader$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StreamReader$len(vecPtr)
    }
}


public class ImageOcrResult: ImageOcrResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageOcrResult$_free(ptr)
        }
    }
}
public class ImageOcrResultRefMut: ImageOcrResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageOcrResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageOcrResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$ImageOcrResult$content(ptr))
    }
}
extension ImageOcrResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageOcrResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageOcrResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageOcrResult) {
        __swift_bridge__$Vec_ImageOcrResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageOcrResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageOcrResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageOcrResultRef> {
        let pointer = __swift_bridge__$Vec_ImageOcrResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageOcrResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageOcrResultRefMut> {
        let pointer = __swift_bridge__$Vec_ImageOcrResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageOcrResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageOcrResultRef> {
        UnsafePointer<ImageOcrResultRef>(OpaquePointer(__swift_bridge__$Vec_ImageOcrResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageOcrResult$len(vecPtr)
    }
}


public class HtmlExtractionResult: HtmlExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HtmlExtractionResult$_free(ptr)
        }
    }
}
public class HtmlExtractionResultRefMut: HtmlExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HtmlExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HtmlExtractionResultRef {
    public func markdown() -> RustString {
        RustString(ptr: __swift_bridge__$HtmlExtractionResult$markdown(ptr))
    }

    public func images() -> RustVec<ExtractedInlineImage> {
        RustVec(ptr: __swift_bridge__$HtmlExtractionResult$images(ptr))
    }

    public func warnings() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$HtmlExtractionResult$warnings(ptr))
    }
}
extension HtmlExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HtmlExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HtmlExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HtmlExtractionResult) {
        __swift_bridge__$Vec_HtmlExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HtmlExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HtmlExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_HtmlExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_HtmlExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HtmlExtractionResultRef> {
        UnsafePointer<HtmlExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_HtmlExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HtmlExtractionResult$len(vecPtr)
    }
}


public class ExtractedInlineImage: ExtractedInlineImageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractedInlineImage$_free(ptr)
        }
    }
}
public class ExtractedInlineImageRefMut: ExtractedInlineImageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractedInlineImageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractedInlineImageRef {
    public func data() -> RustVec<UInt8> {
        RustVec(ptr: __swift_bridge__$ExtractedInlineImage$data(ptr))
    }

    public func format() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractedInlineImage$format(ptr))
    }

    public func filename() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedInlineImage$filename(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func description() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedInlineImage$description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func dimensions() -> Optional<RustVec<UInt32>> {
        { let val = __swift_bridge__$ExtractedInlineImage$dimensions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func attributes() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$ExtractedInlineImage$attributes(ptr))
    }
}
extension ExtractedInlineImage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractedInlineImage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractedInlineImage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractedInlineImage) {
        __swift_bridge__$Vec_ExtractedInlineImage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractedInlineImage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractedInlineImage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractedInlineImageRef> {
        let pointer = __swift_bridge__$Vec_ExtractedInlineImage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractedInlineImageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractedInlineImageRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractedInlineImage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractedInlineImageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractedInlineImageRef> {
        UnsafePointer<ExtractedInlineImageRef>(OpaquePointer(__swift_bridge__$Vec_ExtractedInlineImage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractedInlineImage$len(vecPtr)
    }
}


public class Drawing: DrawingRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Drawing$_free(ptr)
        }
    }
}
extension Drawing {
    public convenience init<GenericIntoRustString: IntoRustString>(_ drawing_type: GenericIntoRustString, _ extent: Optional<GenericIntoRustString>, _ doc_properties: Optional<GenericIntoRustString>, _ image_ref: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$Drawing$new({ let rustString = drawing_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let rustString = optionalStringIntoRustString(extent) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(doc_properties) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(image_ref) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class DrawingRefMut: DrawingRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DrawingRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DrawingRef {
    public func drawing_type() -> RustString {
        RustString(ptr: __swift_bridge__$Drawing$drawing_type(ptr))
    }

    public func extent() -> Optional<RustString> {
        { let val = __swift_bridge__$Drawing$extent(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func doc_properties() -> Optional<RustString> {
        { let val = __swift_bridge__$Drawing$doc_properties(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func image_ref() -> Optional<RustString> {
        { let val = __swift_bridge__$Drawing$image_ref(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension Drawing: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Drawing$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Drawing$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Drawing) {
        __swift_bridge__$Vec_Drawing$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Drawing$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Drawing(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DrawingRef> {
        let pointer = __swift_bridge__$Vec_Drawing$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DrawingRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DrawingRefMut> {
        let pointer = __swift_bridge__$Vec_Drawing$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DrawingRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DrawingRef> {
        UnsafePointer<DrawingRef>(OpaquePointer(__swift_bridge__$Vec_Drawing$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Drawing$len(vecPtr)
    }
}


public class AnchorProperties: AnchorPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$AnchorProperties$_free(ptr)
        }
    }
}
extension AnchorProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ behind_doc: Bool, _ layout_in_cell: Bool, _ relative_height: Optional<Int64>, _ position_h: Optional<GenericIntoRustString>, _ position_v: Optional<GenericIntoRustString>, _ wrap_type: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$AnchorProperties$new(behind_doc, layout_in_cell, relative_height.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(position_h) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(position_v) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let rustString = wrap_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class AnchorPropertiesRefMut: AnchorPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class AnchorPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension AnchorPropertiesRef {
    public func behind_doc() -> Bool {
        __swift_bridge__$AnchorProperties$behind_doc(ptr)
    }

    public func layout_in_cell() -> Bool {
        __swift_bridge__$AnchorProperties$layout_in_cell(ptr)
    }

    public func relative_height() -> Optional<Int64> {
        __swift_bridge__$AnchorProperties$relative_height(ptr).intoSwiftRepr()
    }

    public func position_h() -> Optional<RustString> {
        { let val = __swift_bridge__$AnchorProperties$position_h(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func position_v() -> Optional<RustString> {
        { let val = __swift_bridge__$AnchorProperties$position_v(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func wrap_type() -> RustString {
        RustString(ptr: __swift_bridge__$AnchorProperties$wrap_type(ptr))
    }
}
extension AnchorProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_AnchorProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_AnchorProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: AnchorProperties) {
        __swift_bridge__$Vec_AnchorProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_AnchorProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (AnchorProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AnchorPropertiesRef> {
        let pointer = __swift_bridge__$Vec_AnchorProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AnchorPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AnchorPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_AnchorProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AnchorPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<AnchorPropertiesRef> {
        UnsafePointer<AnchorPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_AnchorProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_AnchorProperties$len(vecPtr)
    }
}


public class PageMarginsPoints: PageMarginsPointsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageMarginsPoints$_free(ptr)
        }
    }
}
extension PageMarginsPoints {
    public convenience init(_ top: Optional<Double>, _ right: Optional<Double>, _ bottom: Optional<Double>, _ left: Optional<Double>, _ header: Optional<Double>, _ footer: Optional<Double>, _ gutter: Optional<Double>) {
        self.init(ptr: __swift_bridge__$PageMarginsPoints$new(top.intoFfiRepr(), right.intoFfiRepr(), bottom.intoFfiRepr(), left.intoFfiRepr(), header.intoFfiRepr(), footer.intoFfiRepr(), gutter.intoFfiRepr()))
    }
}
public class PageMarginsPointsRefMut: PageMarginsPointsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageMarginsPointsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageMarginsPointsRef {
    public func top() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$top(ptr).intoSwiftRepr()
    }

    public func right() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$right(ptr).intoSwiftRepr()
    }

    public func bottom() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$bottom(ptr).intoSwiftRepr()
    }

    public func left() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$left(ptr).intoSwiftRepr()
    }

    public func header() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$header(ptr).intoSwiftRepr()
    }

    public func footer() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$footer(ptr).intoSwiftRepr()
    }

    public func gutter() -> Optional<Double> {
        __swift_bridge__$PageMarginsPoints$gutter(ptr).intoSwiftRepr()
    }
}
extension PageMarginsPoints: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageMarginsPoints$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageMarginsPoints$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageMarginsPoints) {
        __swift_bridge__$Vec_PageMarginsPoints$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageMarginsPoints$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageMarginsPoints(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageMarginsPointsRef> {
        let pointer = __swift_bridge__$Vec_PageMarginsPoints$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageMarginsPointsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageMarginsPointsRefMut> {
        let pointer = __swift_bridge__$Vec_PageMarginsPoints$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageMarginsPointsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageMarginsPointsRef> {
        UnsafePointer<PageMarginsPointsRef>(OpaquePointer(__swift_bridge__$Vec_PageMarginsPoints$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageMarginsPoints$len(vecPtr)
    }
}


public class StyleDefinition: StyleDefinitionRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StyleDefinition$_free(ptr)
        }
    }
}
public class StyleDefinitionRefMut: StyleDefinitionRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StyleDefinitionRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StyleDefinitionRef {
    public func id() -> RustString {
        RustString(ptr: __swift_bridge__$StyleDefinition$id(ptr))
    }

    public func name() -> Optional<RustString> {
        { let val = __swift_bridge__$StyleDefinition$name(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func style_type() -> RustString {
        RustString(ptr: __swift_bridge__$StyleDefinition$style_type(ptr))
    }

    public func based_on() -> Optional<RustString> {
        { let val = __swift_bridge__$StyleDefinition$based_on(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func next_style() -> Optional<RustString> {
        { let val = __swift_bridge__$StyleDefinition$next_style(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func is_default() -> Bool {
        __swift_bridge__$StyleDefinition$is_default(ptr)
    }

    public func paragraph_properties() -> RustString {
        RustString(ptr: __swift_bridge__$StyleDefinition$paragraph_properties(ptr))
    }

    public func run_properties() -> RustString {
        RustString(ptr: __swift_bridge__$StyleDefinition$run_properties(ptr))
    }
}
extension StyleDefinition: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StyleDefinition$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StyleDefinition$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StyleDefinition) {
        __swift_bridge__$Vec_StyleDefinition$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StyleDefinition$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StyleDefinition(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StyleDefinitionRef> {
        let pointer = __swift_bridge__$Vec_StyleDefinition$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StyleDefinitionRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StyleDefinitionRefMut> {
        let pointer = __swift_bridge__$Vec_StyleDefinition$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StyleDefinitionRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StyleDefinitionRef> {
        UnsafePointer<StyleDefinitionRef>(OpaquePointer(__swift_bridge__$Vec_StyleDefinition$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StyleDefinition$len(vecPtr)
    }
}


public class ResolvedStyle: ResolvedStyleRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ResolvedStyle$_free(ptr)
        }
    }
}
extension ResolvedStyle {
    public convenience init<GenericIntoRustString: IntoRustString>(_ paragraph_properties: GenericIntoRustString, _ run_properties: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$ResolvedStyle$new({ let rustString = paragraph_properties.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = run_properties.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class ResolvedStyleRefMut: ResolvedStyleRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ResolvedStyleRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ResolvedStyleRef {
    public func paragraph_properties() -> RustString {
        RustString(ptr: __swift_bridge__$ResolvedStyle$paragraph_properties(ptr))
    }

    public func run_properties() -> RustString {
        RustString(ptr: __swift_bridge__$ResolvedStyle$run_properties(ptr))
    }
}
extension ResolvedStyle: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ResolvedStyle$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ResolvedStyle$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ResolvedStyle) {
        __swift_bridge__$Vec_ResolvedStyle$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ResolvedStyle$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ResolvedStyle(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ResolvedStyleRef> {
        let pointer = __swift_bridge__$Vec_ResolvedStyle$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ResolvedStyleRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ResolvedStyleRefMut> {
        let pointer = __swift_bridge__$Vec_ResolvedStyle$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ResolvedStyleRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ResolvedStyleRef> {
        UnsafePointer<ResolvedStyleRef>(OpaquePointer(__swift_bridge__$Vec_ResolvedStyle$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ResolvedStyle$len(vecPtr)
    }
}


public class TableProperties: TablePropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TableProperties$_free(ptr)
        }
    }
}
extension TableProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ style_id: Optional<GenericIntoRustString>, _ width: Optional<GenericIntoRustString>, _ alignment: Optional<GenericIntoRustString>, _ layout: Optional<GenericIntoRustString>, _ look: Optional<GenericIntoRustString>, _ borders: Optional<GenericIntoRustString>, _ cell_margins: Optional<GenericIntoRustString>, _ indent: Optional<GenericIntoRustString>, _ caption: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$TableProperties$new({ if let rustString = optionalStringIntoRustString(style_id) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(width) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(alignment) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(layout) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(look) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(borders) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(cell_margins) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(indent) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(caption) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class TablePropertiesRefMut: TablePropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TablePropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TablePropertiesRef {
    public func style_id() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$style_id(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func width() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$width(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func alignment() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$alignment(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func layout() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$layout(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func look() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$look(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func borders() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$borders(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func cell_margins() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$cell_margins(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func indent() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$indent(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func caption() -> Optional<RustString> {
        { let val = __swift_bridge__$TableProperties$caption(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension TableProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TableProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TableProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TableProperties) {
        __swift_bridge__$Vec_TableProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TableProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TableProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TablePropertiesRef> {
        let pointer = __swift_bridge__$Vec_TableProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TablePropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TablePropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_TableProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TablePropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TablePropertiesRef> {
        UnsafePointer<TablePropertiesRef>(OpaquePointer(__swift_bridge__$Vec_TableProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TableProperties$len(vecPtr)
    }
}


public class DocxAppProperties: DocxAppPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DocxAppProperties$_free(ptr)
        }
    }
}
extension DocxAppProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ application: Optional<GenericIntoRustString>, _ app_version: Optional<GenericIntoRustString>, _ template: Optional<GenericIntoRustString>, _ total_time: Optional<Int32>, _ pages: Optional<Int32>, _ words: Optional<Int32>, _ characters: Optional<Int32>, _ characters_with_spaces: Optional<Int32>, _ lines: Optional<Int32>, _ paragraphs: Optional<Int32>, _ company: Optional<GenericIntoRustString>, _ doc_security: Optional<Int32>, _ scale_crop: Optional<Bool>, _ links_up_to_date: Optional<Bool>, _ shared_doc: Optional<Bool>, _ hyperlinks_changed: Optional<Bool>) {
        self.init(ptr: __swift_bridge__$DocxAppProperties$new({ if let rustString = optionalStringIntoRustString(application) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(app_version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(template) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), total_time.intoFfiRepr(), pages.intoFfiRepr(), words.intoFfiRepr(), characters.intoFfiRepr(), characters_with_spaces.intoFfiRepr(), lines.intoFfiRepr(), paragraphs.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(company) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), doc_security.intoFfiRepr(), scale_crop.intoFfiRepr(), links_up_to_date.intoFfiRepr(), shared_doc.intoFfiRepr(), hyperlinks_changed.intoFfiRepr()))
    }
}
public class DocxAppPropertiesRefMut: DocxAppPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DocxAppPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DocxAppPropertiesRef {
    public func application() -> Optional<RustString> {
        { let val = __swift_bridge__$DocxAppProperties$application(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func app_version() -> Optional<RustString> {
        { let val = __swift_bridge__$DocxAppProperties$app_version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func template() -> Optional<RustString> {
        { let val = __swift_bridge__$DocxAppProperties$template(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func total_time() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$total_time(ptr).intoSwiftRepr()
    }

    public func pages() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$pages(ptr).intoSwiftRepr()
    }

    public func words() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$words(ptr).intoSwiftRepr()
    }

    public func characters() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$characters(ptr).intoSwiftRepr()
    }

    public func characters_with_spaces() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$characters_with_spaces(ptr).intoSwiftRepr()
    }

    public func lines() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$lines(ptr).intoSwiftRepr()
    }

    public func paragraphs() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$paragraphs(ptr).intoSwiftRepr()
    }

    public func company() -> Optional<RustString> {
        { let val = __swift_bridge__$DocxAppProperties$company(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func doc_security() -> Optional<Int32> {
        __swift_bridge__$DocxAppProperties$doc_security(ptr).intoSwiftRepr()
    }

    public func scale_crop() -> Optional<Bool> {
        __swift_bridge__$DocxAppProperties$scale_crop(ptr).intoSwiftRepr()
    }

    public func links_up_to_date() -> Optional<Bool> {
        __swift_bridge__$DocxAppProperties$links_up_to_date(ptr).intoSwiftRepr()
    }

    public func shared_doc() -> Optional<Bool> {
        __swift_bridge__$DocxAppProperties$shared_doc(ptr).intoSwiftRepr()
    }

    public func hyperlinks_changed() -> Optional<Bool> {
        __swift_bridge__$DocxAppProperties$hyperlinks_changed(ptr).intoSwiftRepr()
    }
}
extension DocxAppProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DocxAppProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DocxAppProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DocxAppProperties) {
        __swift_bridge__$Vec_DocxAppProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DocxAppProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DocxAppProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocxAppPropertiesRef> {
        let pointer = __swift_bridge__$Vec_DocxAppProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocxAppPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocxAppPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_DocxAppProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocxAppPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DocxAppPropertiesRef> {
        UnsafePointer<DocxAppPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_DocxAppProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DocxAppProperties$len(vecPtr)
    }
}


public class XlsxAppProperties: XlsxAppPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$XlsxAppProperties$_free(ptr)
        }
    }
}
extension XlsxAppProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ application: Optional<GenericIntoRustString>, _ app_version: Optional<GenericIntoRustString>, _ doc_security: Optional<Int32>, _ scale_crop: Optional<Bool>, _ links_up_to_date: Optional<Bool>, _ shared_doc: Optional<Bool>, _ hyperlinks_changed: Optional<Bool>, _ company: Optional<GenericIntoRustString>, _ worksheet_names: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$XlsxAppProperties$new({ if let rustString = optionalStringIntoRustString(application) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(app_version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), doc_security.intoFfiRepr(), scale_crop.intoFfiRepr(), links_up_to_date.intoFfiRepr(), shared_doc.intoFfiRepr(), hyperlinks_changed.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(company) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = worksheet_names; val.isOwned = false; return val.ptr }()))
    }
}
public class XlsxAppPropertiesRefMut: XlsxAppPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class XlsxAppPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension XlsxAppPropertiesRef {
    public func application() -> Optional<RustString> {
        { let val = __swift_bridge__$XlsxAppProperties$application(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func app_version() -> Optional<RustString> {
        { let val = __swift_bridge__$XlsxAppProperties$app_version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func doc_security() -> Optional<Int32> {
        __swift_bridge__$XlsxAppProperties$doc_security(ptr).intoSwiftRepr()
    }

    public func scale_crop() -> Optional<Bool> {
        __swift_bridge__$XlsxAppProperties$scale_crop(ptr).intoSwiftRepr()
    }

    public func links_up_to_date() -> Optional<Bool> {
        __swift_bridge__$XlsxAppProperties$links_up_to_date(ptr).intoSwiftRepr()
    }

    public func shared_doc() -> Optional<Bool> {
        __swift_bridge__$XlsxAppProperties$shared_doc(ptr).intoSwiftRepr()
    }

    public func hyperlinks_changed() -> Optional<Bool> {
        __swift_bridge__$XlsxAppProperties$hyperlinks_changed(ptr).intoSwiftRepr()
    }

    public func company() -> Optional<RustString> {
        { let val = __swift_bridge__$XlsxAppProperties$company(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func worksheet_names() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$XlsxAppProperties$worksheet_names(ptr))
    }
}
extension XlsxAppProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_XlsxAppProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_XlsxAppProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: XlsxAppProperties) {
        __swift_bridge__$Vec_XlsxAppProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_XlsxAppProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (XlsxAppProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XlsxAppPropertiesRef> {
        let pointer = __swift_bridge__$Vec_XlsxAppProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XlsxAppPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XlsxAppPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_XlsxAppProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XlsxAppPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<XlsxAppPropertiesRef> {
        UnsafePointer<XlsxAppPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_XlsxAppProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_XlsxAppProperties$len(vecPtr)
    }
}


public class PptxAppProperties: PptxAppPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PptxAppProperties$_free(ptr)
        }
    }
}
extension PptxAppProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ application: Optional<GenericIntoRustString>, _ app_version: Optional<GenericIntoRustString>, _ total_time: Optional<Int32>, _ company: Optional<GenericIntoRustString>, _ doc_security: Optional<Int32>, _ scale_crop: Optional<Bool>, _ links_up_to_date: Optional<Bool>, _ shared_doc: Optional<Bool>, _ hyperlinks_changed: Optional<Bool>, _ slides: Optional<Int32>, _ notes: Optional<Int32>, _ hidden_slides: Optional<Int32>, _ multimedia_clips: Optional<Int32>, _ presentation_format: Optional<GenericIntoRustString>, _ slide_titles: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$PptxAppProperties$new({ if let rustString = optionalStringIntoRustString(application) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(app_version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), total_time.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(company) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), doc_security.intoFfiRepr(), scale_crop.intoFfiRepr(), links_up_to_date.intoFfiRepr(), shared_doc.intoFfiRepr(), hyperlinks_changed.intoFfiRepr(), slides.intoFfiRepr(), notes.intoFfiRepr(), hidden_slides.intoFfiRepr(), multimedia_clips.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(presentation_format) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = slide_titles; val.isOwned = false; return val.ptr }()))
    }
}
public class PptxAppPropertiesRefMut: PptxAppPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PptxAppPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PptxAppPropertiesRef {
    public func application() -> Optional<RustString> {
        { let val = __swift_bridge__$PptxAppProperties$application(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func app_version() -> Optional<RustString> {
        { let val = __swift_bridge__$PptxAppProperties$app_version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func total_time() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$total_time(ptr).intoSwiftRepr()
    }

    public func company() -> Optional<RustString> {
        { let val = __swift_bridge__$PptxAppProperties$company(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func doc_security() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$doc_security(ptr).intoSwiftRepr()
    }

    public func scale_crop() -> Optional<Bool> {
        __swift_bridge__$PptxAppProperties$scale_crop(ptr).intoSwiftRepr()
    }

    public func links_up_to_date() -> Optional<Bool> {
        __swift_bridge__$PptxAppProperties$links_up_to_date(ptr).intoSwiftRepr()
    }

    public func shared_doc() -> Optional<Bool> {
        __swift_bridge__$PptxAppProperties$shared_doc(ptr).intoSwiftRepr()
    }

    public func hyperlinks_changed() -> Optional<Bool> {
        __swift_bridge__$PptxAppProperties$hyperlinks_changed(ptr).intoSwiftRepr()
    }

    public func slides() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$slides(ptr).intoSwiftRepr()
    }

    public func notes() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$notes(ptr).intoSwiftRepr()
    }

    public func hidden_slides() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$hidden_slides(ptr).intoSwiftRepr()
    }

    public func multimedia_clips() -> Optional<Int32> {
        __swift_bridge__$PptxAppProperties$multimedia_clips(ptr).intoSwiftRepr()
    }

    public func presentation_format() -> Optional<RustString> {
        { let val = __swift_bridge__$PptxAppProperties$presentation_format(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func slide_titles() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$PptxAppProperties$slide_titles(ptr))
    }
}
extension PptxAppProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PptxAppProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PptxAppProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PptxAppProperties) {
        __swift_bridge__$Vec_PptxAppProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PptxAppProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PptxAppProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxAppPropertiesRef> {
        let pointer = __swift_bridge__$Vec_PptxAppProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxAppPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxAppPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_PptxAppProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxAppPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PptxAppPropertiesRef> {
        UnsafePointer<PptxAppPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_PptxAppProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PptxAppProperties$len(vecPtr)
    }
}


public class CoreProperties: CorePropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CoreProperties$_free(ptr)
        }
    }
}
extension CoreProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ title: Optional<GenericIntoRustString>, _ subject: Optional<GenericIntoRustString>, _ creator: Optional<GenericIntoRustString>, _ keywords: Optional<GenericIntoRustString>, _ description: Optional<GenericIntoRustString>, _ last_modified_by: Optional<GenericIntoRustString>, _ revision: Optional<GenericIntoRustString>, _ created: Optional<GenericIntoRustString>, _ modified: Optional<GenericIntoRustString>, _ category: Optional<GenericIntoRustString>, _ content_status: Optional<GenericIntoRustString>, _ language: Optional<GenericIntoRustString>, _ identifier: Optional<GenericIntoRustString>, _ version: Optional<GenericIntoRustString>, _ last_printed: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$CoreProperties$new({ if let rustString = optionalStringIntoRustString(title) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(subject) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(creator) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(keywords) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(description) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(last_modified_by) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(revision) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(created) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(modified) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(category) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(content_status) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(language) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(identifier) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(last_printed) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class CorePropertiesRefMut: CorePropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CorePropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CorePropertiesRef {
    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func subject() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$subject(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func creator() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$creator(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$keywords(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func description() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func last_modified_by() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$last_modified_by(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func revision() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$revision(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func created() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$created(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func modified() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$modified(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func category() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$category(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func content_status() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$content_status(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func identifier() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$identifier(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func version() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func last_printed() -> Optional<RustString> {
        { let val = __swift_bridge__$CoreProperties$last_printed(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension CoreProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CoreProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CoreProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CoreProperties) {
        __swift_bridge__$Vec_CoreProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CoreProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CoreProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CorePropertiesRef> {
        let pointer = __swift_bridge__$Vec_CoreProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CorePropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CorePropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_CoreProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CorePropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CorePropertiesRef> {
        UnsafePointer<CorePropertiesRef>(OpaquePointer(__swift_bridge__$Vec_CoreProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CoreProperties$len(vecPtr)
    }
}


public class CustomProperties: CustomPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CustomProperties$_free(ptr)
        }
    }
}
public class CustomPropertiesRefMut: CustomPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CustomPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CustomProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CustomProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CustomProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CustomProperties) {
        __swift_bridge__$Vec_CustomProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CustomProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CustomProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CustomPropertiesRef> {
        let pointer = __swift_bridge__$Vec_CustomProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CustomPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CustomPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_CustomProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CustomPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CustomPropertiesRef> {
        UnsafePointer<CustomPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_CustomProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CustomProperties$len(vecPtr)
    }
}


public class OdtProperties: OdtPropertiesRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OdtProperties$_free(ptr)
        }
    }
}
extension OdtProperties {
    public convenience init<GenericIntoRustString: IntoRustString>(_ title: Optional<GenericIntoRustString>, _ subject: Optional<GenericIntoRustString>, _ creator: Optional<GenericIntoRustString>, _ initial_creator: Optional<GenericIntoRustString>, _ keywords: Optional<GenericIntoRustString>, _ description: Optional<GenericIntoRustString>, _ date: Optional<GenericIntoRustString>, _ creation_date: Optional<GenericIntoRustString>, _ language: Optional<GenericIntoRustString>, _ generator: Optional<GenericIntoRustString>, _ editing_duration: Optional<GenericIntoRustString>, _ editing_cycles: Optional<GenericIntoRustString>, _ page_count: Optional<Int32>, _ word_count: Optional<Int32>, _ character_count: Optional<Int32>, _ paragraph_count: Optional<Int32>, _ table_count: Optional<Int32>, _ image_count: Optional<Int32>) {
        self.init(ptr: __swift_bridge__$OdtProperties$new({ if let rustString = optionalStringIntoRustString(title) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(subject) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(creator) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(initial_creator) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(keywords) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(description) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(date) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(creation_date) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(language) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(generator) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(editing_duration) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(editing_cycles) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), page_count.intoFfiRepr(), word_count.intoFfiRepr(), character_count.intoFfiRepr(), paragraph_count.intoFfiRepr(), table_count.intoFfiRepr(), image_count.intoFfiRepr()))
    }
}
public class OdtPropertiesRefMut: OdtPropertiesRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OdtPropertiesRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OdtPropertiesRef {
    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func subject() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$subject(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func creator() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$creator(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func initial_creator() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$initial_creator(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$keywords(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func description() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func date() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$date(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func creation_date() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$creation_date(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func generator() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$generator(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func editing_duration() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$editing_duration(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func editing_cycles() -> Optional<RustString> {
        { let val = __swift_bridge__$OdtProperties$editing_cycles(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func page_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$page_count(ptr).intoSwiftRepr()
    }

    public func word_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$word_count(ptr).intoSwiftRepr()
    }

    public func character_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$character_count(ptr).intoSwiftRepr()
    }

    public func paragraph_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$paragraph_count(ptr).intoSwiftRepr()
    }

    public func table_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$table_count(ptr).intoSwiftRepr()
    }

    public func image_count() -> Optional<Int32> {
        __swift_bridge__$OdtProperties$image_count(ptr).intoSwiftRepr()
    }
}
extension OdtProperties: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OdtProperties$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OdtProperties$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OdtProperties) {
        __swift_bridge__$Vec_OdtProperties$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OdtProperties$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OdtProperties(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OdtPropertiesRef> {
        let pointer = __swift_bridge__$Vec_OdtProperties$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OdtPropertiesRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OdtPropertiesRefMut> {
        let pointer = __swift_bridge__$Vec_OdtProperties$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OdtPropertiesRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OdtPropertiesRef> {
        UnsafePointer<OdtPropertiesRef>(OpaquePointer(__swift_bridge__$Vec_OdtProperties$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OdtProperties$len(vecPtr)
    }
}


public class SecurityLimits: SecurityLimitsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$SecurityLimits$_free(ptr)
        }
    }
}
extension SecurityLimits {
    public convenience init(_ max_archive_size: UInt, _ max_compression_ratio: UInt, _ max_files_in_archive: UInt, _ max_nesting_depth: UInt, _ max_entity_length: UInt, _ max_content_size: UInt, _ max_iterations: UInt, _ max_xml_depth: UInt, _ max_table_cells: UInt) {
        self.init(ptr: __swift_bridge__$SecurityLimits$new(max_archive_size, max_compression_ratio, max_files_in_archive, max_nesting_depth, max_entity_length, max_content_size, max_iterations, max_xml_depth, max_table_cells))
    }
}
public class SecurityLimitsRefMut: SecurityLimitsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class SecurityLimitsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension SecurityLimitsRef {
    public func max_archive_size() -> UInt {
        __swift_bridge__$SecurityLimits$max_archive_size(ptr)
    }

    public func max_compression_ratio() -> UInt {
        __swift_bridge__$SecurityLimits$max_compression_ratio(ptr)
    }

    public func max_files_in_archive() -> UInt {
        __swift_bridge__$SecurityLimits$max_files_in_archive(ptr)
    }

    public func max_nesting_depth() -> UInt {
        __swift_bridge__$SecurityLimits$max_nesting_depth(ptr)
    }

    public func max_entity_length() -> UInt {
        __swift_bridge__$SecurityLimits$max_entity_length(ptr)
    }

    public func max_content_size() -> UInt {
        __swift_bridge__$SecurityLimits$max_content_size(ptr)
    }

    public func max_iterations() -> UInt {
        __swift_bridge__$SecurityLimits$max_iterations(ptr)
    }

    public func max_xml_depth() -> UInt {
        __swift_bridge__$SecurityLimits$max_xml_depth(ptr)
    }

    public func max_table_cells() -> UInt {
        __swift_bridge__$SecurityLimits$max_table_cells(ptr)
    }
}
extension SecurityLimits: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_SecurityLimits$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_SecurityLimits$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: SecurityLimits) {
        __swift_bridge__$Vec_SecurityLimits$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_SecurityLimits$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (SecurityLimits(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SecurityLimitsRef> {
        let pointer = __swift_bridge__$Vec_SecurityLimits$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return SecurityLimitsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SecurityLimitsRefMut> {
        let pointer = __swift_bridge__$Vec_SecurityLimits$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return SecurityLimitsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<SecurityLimitsRef> {
        UnsafePointer<SecurityLimitsRef>(OpaquePointer(__swift_bridge__$Vec_SecurityLimits$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_SecurityLimits$len(vecPtr)
    }
}


public class ZipBombValidator: ZipBombValidatorRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ZipBombValidator$_free(ptr)
        }
    }
}
public class ZipBombValidatorRefMut: ZipBombValidatorRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ZipBombValidatorRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ZipBombValidator: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ZipBombValidator$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ZipBombValidator$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ZipBombValidator) {
        __swift_bridge__$Vec_ZipBombValidator$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ZipBombValidator$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ZipBombValidator(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ZipBombValidatorRef> {
        let pointer = __swift_bridge__$Vec_ZipBombValidator$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ZipBombValidatorRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ZipBombValidatorRefMut> {
        let pointer = __swift_bridge__$Vec_ZipBombValidator$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ZipBombValidatorRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ZipBombValidatorRef> {
        UnsafePointer<ZipBombValidatorRef>(OpaquePointer(__swift_bridge__$Vec_ZipBombValidator$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ZipBombValidator$len(vecPtr)
    }
}


public class TokenReductionConfig: TokenReductionConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TokenReductionConfig$_free(ptr)
        }
    }
}
extension TokenReductionConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ level: ReductionLevel, _ language_hint: Optional<GenericIntoRustString>, _ preserve_markdown: Bool, _ preserve_code: Bool, _ semantic_threshold: Float, _ enable_parallel: Bool, _ use_simd: Bool, _ custom_stopwords: GenericIntoRustString, _ preserve_patterns: RustVec<GenericIntoRustString>, _ target_reduction: Optional<Float>, _ enable_semantic_clustering: Bool) {
        self.init(ptr: __swift_bridge__$TokenReductionConfig$new({level.isOwned = false; return level.ptr;}(), { if let rustString = optionalStringIntoRustString(language_hint) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), preserve_markdown, preserve_code, semantic_threshold, enable_parallel, use_simd, { let rustString = custom_stopwords.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = preserve_patterns; val.isOwned = false; return val.ptr }(), target_reduction.intoFfiRepr(), enable_semantic_clustering))
    }
}
public class TokenReductionConfigRefMut: TokenReductionConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TokenReductionConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TokenReductionConfigRef {
    public func level() -> ReductionLevel {
        ReductionLevel(ptr: __swift_bridge__$TokenReductionConfig$level(ptr))
    }

    public func language_hint() -> Optional<RustString> {
        { let val = __swift_bridge__$TokenReductionConfig$language_hint(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func preserve_markdown() -> Bool {
        __swift_bridge__$TokenReductionConfig$preserve_markdown(ptr)
    }

    public func preserve_code() -> Bool {
        __swift_bridge__$TokenReductionConfig$preserve_code(ptr)
    }

    public func semantic_threshold() -> Float {
        __swift_bridge__$TokenReductionConfig$semantic_threshold(ptr)
    }

    public func enable_parallel() -> Bool {
        __swift_bridge__$TokenReductionConfig$enable_parallel(ptr)
    }

    public func use_simd() -> Bool {
        __swift_bridge__$TokenReductionConfig$use_simd(ptr)
    }

    public func custom_stopwords() -> RustString {
        RustString(ptr: __swift_bridge__$TokenReductionConfig$custom_stopwords(ptr))
    }

    public func preserve_patterns() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$TokenReductionConfig$preserve_patterns(ptr))
    }

    public func target_reduction() -> Optional<Float> {
        __swift_bridge__$TokenReductionConfig$target_reduction(ptr).intoSwiftRepr()
    }

    public func enable_semantic_clustering() -> Bool {
        __swift_bridge__$TokenReductionConfig$enable_semantic_clustering(ptr)
    }
}
extension TokenReductionConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TokenReductionConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TokenReductionConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TokenReductionConfig) {
        __swift_bridge__$Vec_TokenReductionConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TokenReductionConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TokenReductionConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TokenReductionConfigRef> {
        let pointer = __swift_bridge__$Vec_TokenReductionConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TokenReductionConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TokenReductionConfigRefMut> {
        let pointer = __swift_bridge__$Vec_TokenReductionConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TokenReductionConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TokenReductionConfigRef> {
        UnsafePointer<TokenReductionConfigRef>(OpaquePointer(__swift_bridge__$Vec_TokenReductionConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TokenReductionConfig$len(vecPtr)
    }
}


public class PdfAnnotation: PdfAnnotationRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PdfAnnotation$_free(ptr)
        }
    }
}
public class PdfAnnotationRefMut: PdfAnnotationRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PdfAnnotationRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PdfAnnotationRef {
    public func annotation_type() -> PdfAnnotationType {
        PdfAnnotationType(ptr: __swift_bridge__$PdfAnnotation$annotation_type(ptr))
    }

    public func content() -> Optional<RustString> {
        { let val = __swift_bridge__$PdfAnnotation$content(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func page_number() -> UInt {
        __swift_bridge__$PdfAnnotation$page_number(ptr)
    }

    public func bounding_box() -> Optional<RustString> {
        { let val = __swift_bridge__$PdfAnnotation$bounding_box(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension PdfAnnotation: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PdfAnnotation$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PdfAnnotation$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PdfAnnotation) {
        __swift_bridge__$Vec_PdfAnnotation$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PdfAnnotation$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PdfAnnotation(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfAnnotationRef> {
        let pointer = __swift_bridge__$Vec_PdfAnnotation$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfAnnotationRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfAnnotationRefMut> {
        let pointer = __swift_bridge__$Vec_PdfAnnotation$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfAnnotationRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PdfAnnotationRef> {
        UnsafePointer<PdfAnnotationRef>(OpaquePointer(__swift_bridge__$Vec_PdfAnnotation$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PdfAnnotation$len(vecPtr)
    }
}


public class DjotContent: DjotContentRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DjotContent$_free(ptr)
        }
    }
}
public class DjotContentRefMut: DjotContentRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DjotContentRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DjotContentRef {
    public func plain_text() -> RustString {
        RustString(ptr: __swift_bridge__$DjotContent$plain_text(ptr))
    }

    public func blocks() -> RustVec<FormattedBlock> {
        RustVec(ptr: __swift_bridge__$DjotContent$blocks(ptr))
    }

    public func metadata() -> Metadata {
        Metadata(ptr: __swift_bridge__$DjotContent$metadata(ptr))
    }

    public func tables() -> RustVec<Table> {
        RustVec(ptr: __swift_bridge__$DjotContent$tables(ptr))
    }

    public func images() -> RustVec<DjotImage> {
        RustVec(ptr: __swift_bridge__$DjotContent$images(ptr))
    }

    public func links() -> RustVec<DjotLink> {
        RustVec(ptr: __swift_bridge__$DjotContent$links(ptr))
    }

    public func footnotes() -> RustVec<Footnote> {
        RustVec(ptr: __swift_bridge__$DjotContent$footnotes(ptr))
    }

    public func attributes() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$DjotContent$attributes(ptr))
    }
}
extension DjotContent: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DjotContent$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DjotContent$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DjotContent) {
        __swift_bridge__$Vec_DjotContent$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DjotContent$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DjotContent(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotContentRef> {
        let pointer = __swift_bridge__$Vec_DjotContent$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotContentRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotContentRefMut> {
        let pointer = __swift_bridge__$Vec_DjotContent$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotContentRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DjotContentRef> {
        UnsafePointer<DjotContentRef>(OpaquePointer(__swift_bridge__$Vec_DjotContent$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DjotContent$len(vecPtr)
    }
}


public class FormattedBlock: FormattedBlockRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$FormattedBlock$_free(ptr)
        }
    }
}
public class FormattedBlockRefMut: FormattedBlockRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FormattedBlockRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FormattedBlockRef {
    public func block_type() -> BlockType {
        BlockType(ptr: __swift_bridge__$FormattedBlock$block_type(ptr))
    }

    public func level() -> Optional<UInt> {
        __swift_bridge__$FormattedBlock$level(ptr).intoSwiftRepr()
    }

    public func inline_content() -> RustVec<InlineElement> {
        RustVec(ptr: __swift_bridge__$FormattedBlock$inline_content(ptr))
    }

    public func attributes() -> Optional<RustString> {
        { let val = __swift_bridge__$FormattedBlock$attributes(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$FormattedBlock$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func code() -> Optional<RustString> {
        { let val = __swift_bridge__$FormattedBlock$code(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func children() -> RustVec<FormattedBlock> {
        RustVec(ptr: __swift_bridge__$FormattedBlock$children(ptr))
    }
}
extension FormattedBlock: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_FormattedBlock$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_FormattedBlock$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: FormattedBlock) {
        __swift_bridge__$Vec_FormattedBlock$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_FormattedBlock$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (FormattedBlock(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FormattedBlockRef> {
        let pointer = __swift_bridge__$Vec_FormattedBlock$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FormattedBlockRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FormattedBlockRefMut> {
        let pointer = __swift_bridge__$Vec_FormattedBlock$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FormattedBlockRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FormattedBlockRef> {
        UnsafePointer<FormattedBlockRef>(OpaquePointer(__swift_bridge__$Vec_FormattedBlock$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_FormattedBlock$len(vecPtr)
    }
}


public class InlineElement: InlineElementRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$InlineElement$_free(ptr)
        }
    }
}
public class InlineElementRefMut: InlineElementRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class InlineElementRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension InlineElementRef {
    public func element_type() -> InlineType {
        InlineType(ptr: __swift_bridge__$InlineElement$element_type(ptr))
    }

    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$InlineElement$content(ptr))
    }

    public func attributes() -> Optional<RustString> {
        { let val = __swift_bridge__$InlineElement$attributes(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$InlineElement$metadata(ptr))
    }
}
extension InlineElement: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_InlineElement$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_InlineElement$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: InlineElement) {
        __swift_bridge__$Vec_InlineElement$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_InlineElement$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (InlineElement(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InlineElementRef> {
        let pointer = __swift_bridge__$Vec_InlineElement$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InlineElementRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InlineElementRefMut> {
        let pointer = __swift_bridge__$Vec_InlineElement$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InlineElementRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<InlineElementRef> {
        UnsafePointer<InlineElementRef>(OpaquePointer(__swift_bridge__$Vec_InlineElement$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_InlineElement$len(vecPtr)
    }
}


public class DjotImage: DjotImageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DjotImage$_free(ptr)
        }
    }
}
public class DjotImageRefMut: DjotImageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DjotImageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DjotImageRef {
    public func src() -> RustString {
        RustString(ptr: __swift_bridge__$DjotImage$src(ptr))
    }

    public func alt() -> RustString {
        RustString(ptr: __swift_bridge__$DjotImage$alt(ptr))
    }

    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$DjotImage$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func attributes() -> Optional<RustString> {
        { let val = __swift_bridge__$DjotImage$attributes(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension DjotImage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DjotImage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DjotImage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DjotImage) {
        __swift_bridge__$Vec_DjotImage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DjotImage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DjotImage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotImageRef> {
        let pointer = __swift_bridge__$Vec_DjotImage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotImageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotImageRefMut> {
        let pointer = __swift_bridge__$Vec_DjotImage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotImageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DjotImageRef> {
        UnsafePointer<DjotImageRef>(OpaquePointer(__swift_bridge__$Vec_DjotImage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DjotImage$len(vecPtr)
    }
}


public class DjotLink: DjotLinkRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DjotLink$_free(ptr)
        }
    }
}
public class DjotLinkRefMut: DjotLinkRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DjotLinkRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DjotLinkRef {
    public func url() -> RustString {
        RustString(ptr: __swift_bridge__$DjotLink$url(ptr))
    }

    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$DjotLink$text(ptr))
    }

    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$DjotLink$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func attributes() -> Optional<RustString> {
        { let val = __swift_bridge__$DjotLink$attributes(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension DjotLink: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DjotLink$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DjotLink$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DjotLink) {
        __swift_bridge__$Vec_DjotLink$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DjotLink$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DjotLink(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotLinkRef> {
        let pointer = __swift_bridge__$Vec_DjotLink$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotLinkRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DjotLinkRefMut> {
        let pointer = __swift_bridge__$Vec_DjotLink$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DjotLinkRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DjotLinkRef> {
        UnsafePointer<DjotLinkRef>(OpaquePointer(__swift_bridge__$Vec_DjotLink$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DjotLink$len(vecPtr)
    }
}


public class Footnote: FootnoteRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Footnote$_free(ptr)
        }
    }
}
public class FootnoteRefMut: FootnoteRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FootnoteRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FootnoteRef {
    public func label() -> RustString {
        RustString(ptr: __swift_bridge__$Footnote$label(ptr))
    }

    public func content() -> RustVec<FormattedBlock> {
        RustVec(ptr: __swift_bridge__$Footnote$content(ptr))
    }
}
extension Footnote: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Footnote$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Footnote$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Footnote) {
        __swift_bridge__$Vec_Footnote$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Footnote$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Footnote(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FootnoteRef> {
        let pointer = __swift_bridge__$Vec_Footnote$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FootnoteRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FootnoteRefMut> {
        let pointer = __swift_bridge__$Vec_Footnote$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FootnoteRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FootnoteRef> {
        UnsafePointer<FootnoteRef>(OpaquePointer(__swift_bridge__$Vec_Footnote$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Footnote$len(vecPtr)
    }
}


public class DocumentStructure: DocumentStructureRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DocumentStructure$_free(ptr)
        }
    }
}
extension DocumentStructure {
    public convenience init<GenericIntoRustString: IntoRustString>(_ nodes: RustVec<DocumentNode>, _ source_format: Optional<GenericIntoRustString>, _ relationships: RustVec<DocumentRelationship>, _ node_types: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$DocumentStructure$new({ let val = nodes; val.isOwned = false; return val.ptr }(), { if let rustString = optionalStringIntoRustString(source_format) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = relationships; val.isOwned = false; return val.ptr }(), { let val = node_types; val.isOwned = false; return val.ptr }()))
    }
}
public class DocumentStructureRefMut: DocumentStructureRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DocumentStructureRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DocumentStructureRef {
    public func nodes() -> RustVec<DocumentNode> {
        RustVec(ptr: __swift_bridge__$DocumentStructure$nodes(ptr))
    }

    public func source_format() -> Optional<RustString> {
        { let val = __swift_bridge__$DocumentStructure$source_format(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func relationships() -> RustVec<DocumentRelationship> {
        RustVec(ptr: __swift_bridge__$DocumentStructure$relationships(ptr))
    }

    public func node_types() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$DocumentStructure$node_types(ptr))
    }
}
extension DocumentStructure: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DocumentStructure$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DocumentStructure$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DocumentStructure) {
        __swift_bridge__$Vec_DocumentStructure$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DocumentStructure$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DocumentStructure(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentStructureRef> {
        let pointer = __swift_bridge__$Vec_DocumentStructure$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentStructureRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentStructureRefMut> {
        let pointer = __swift_bridge__$Vec_DocumentStructure$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentStructureRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DocumentStructureRef> {
        UnsafePointer<DocumentStructureRef>(OpaquePointer(__swift_bridge__$Vec_DocumentStructure$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DocumentStructure$len(vecPtr)
    }
}


public class DocumentRelationship: DocumentRelationshipRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DocumentRelationship$_free(ptr)
        }
    }
}
public class DocumentRelationshipRefMut: DocumentRelationshipRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DocumentRelationshipRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DocumentRelationshipRef {
    public func source() -> UInt32 {
        __swift_bridge__$DocumentRelationship$source(ptr)
    }

    public func target() -> UInt32 {
        __swift_bridge__$DocumentRelationship$target(ptr)
    }

    public func kind() -> RelationshipKind {
        RelationshipKind(ptr: __swift_bridge__$DocumentRelationship$kind(ptr))
    }
}
extension DocumentRelationship: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DocumentRelationship$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DocumentRelationship$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DocumentRelationship) {
        __swift_bridge__$Vec_DocumentRelationship$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DocumentRelationship$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DocumentRelationship(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentRelationshipRef> {
        let pointer = __swift_bridge__$Vec_DocumentRelationship$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentRelationshipRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentRelationshipRefMut> {
        let pointer = __swift_bridge__$Vec_DocumentRelationship$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentRelationshipRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DocumentRelationshipRef> {
        UnsafePointer<DocumentRelationshipRef>(OpaquePointer(__swift_bridge__$Vec_DocumentRelationship$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DocumentRelationship$len(vecPtr)
    }
}


public class DocumentNode: DocumentNodeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DocumentNode$_free(ptr)
        }
    }
}
public class DocumentNodeRefMut: DocumentNodeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DocumentNodeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DocumentNodeRef {
    public func id() -> RustString {
        RustString(ptr: __swift_bridge__$DocumentNode$id(ptr))
    }

    public func content() -> NodeContent {
        NodeContent(ptr: __swift_bridge__$DocumentNode$content(ptr))
    }

    public func parent() -> Optional<UInt32> {
        __swift_bridge__$DocumentNode$parent(ptr).intoSwiftRepr()
    }

    public func children() -> RustVec<UInt32> {
        RustVec(ptr: __swift_bridge__$DocumentNode$children(ptr))
    }

    public func content_layer() -> ContentLayer {
        ContentLayer(ptr: __swift_bridge__$DocumentNode$content_layer(ptr))
    }

    public func page() -> Optional<UInt32> {
        __swift_bridge__$DocumentNode$page(ptr).intoSwiftRepr()
    }

    public func page_end() -> Optional<UInt32> {
        __swift_bridge__$DocumentNode$page_end(ptr).intoSwiftRepr()
    }

    public func bbox() -> Optional<RustString> {
        { let val = __swift_bridge__$DocumentNode$bbox(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func annotations() -> RustVec<TextAnnotation> {
        RustVec(ptr: __swift_bridge__$DocumentNode$annotations(ptr))
    }

    public func attributes() -> RustString {
        RustString(ptr: __swift_bridge__$DocumentNode$attributes(ptr))
    }
}
extension DocumentNode: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DocumentNode$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DocumentNode$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DocumentNode) {
        __swift_bridge__$Vec_DocumentNode$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DocumentNode$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DocumentNode(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentNodeRef> {
        let pointer = __swift_bridge__$Vec_DocumentNode$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentNodeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocumentNodeRefMut> {
        let pointer = __swift_bridge__$Vec_DocumentNode$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocumentNodeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DocumentNodeRef> {
        UnsafePointer<DocumentNodeRef>(OpaquePointer(__swift_bridge__$Vec_DocumentNode$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DocumentNode$len(vecPtr)
    }
}


public class TableGrid: TableGridRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TableGrid$_free(ptr)
        }
    }
}
extension TableGrid {
    public convenience init(_ rows: UInt32, _ cols: UInt32, _ cells: RustVec<GridCell>) {
        self.init(ptr: __swift_bridge__$TableGrid$new(rows, cols, { let val = cells; val.isOwned = false; return val.ptr }()))
    }
}
public class TableGridRefMut: TableGridRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TableGridRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TableGridRef {
    public func rows() -> UInt32 {
        __swift_bridge__$TableGrid$rows(ptr)
    }

    public func cols() -> UInt32 {
        __swift_bridge__$TableGrid$cols(ptr)
    }

    public func cells() -> RustVec<GridCell> {
        RustVec(ptr: __swift_bridge__$TableGrid$cells(ptr))
    }
}
extension TableGrid: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TableGrid$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TableGrid$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TableGrid) {
        __swift_bridge__$Vec_TableGrid$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TableGrid$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TableGrid(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableGridRef> {
        let pointer = __swift_bridge__$Vec_TableGrid$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableGridRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableGridRefMut> {
        let pointer = __swift_bridge__$Vec_TableGrid$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableGridRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TableGridRef> {
        UnsafePointer<TableGridRef>(OpaquePointer(__swift_bridge__$Vec_TableGrid$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TableGrid$len(vecPtr)
    }
}


public class GridCell: GridCellRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$GridCell$_free(ptr)
        }
    }
}
public class GridCellRefMut: GridCellRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class GridCellRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension GridCellRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$GridCell$content(ptr))
    }

    public func row() -> UInt32 {
        __swift_bridge__$GridCell$row(ptr)
    }

    public func col() -> UInt32 {
        __swift_bridge__$GridCell$col(ptr)
    }

    public func row_span() -> UInt32 {
        __swift_bridge__$GridCell$row_span(ptr)
    }

    public func col_span() -> UInt32 {
        __swift_bridge__$GridCell$col_span(ptr)
    }

    public func is_header() -> Bool {
        __swift_bridge__$GridCell$is_header(ptr)
    }

    public func bbox() -> Optional<RustString> {
        { let val = __swift_bridge__$GridCell$bbox(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension GridCell: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_GridCell$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_GridCell$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: GridCell) {
        __swift_bridge__$Vec_GridCell$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_GridCell$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (GridCell(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<GridCellRef> {
        let pointer = __swift_bridge__$Vec_GridCell$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return GridCellRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<GridCellRefMut> {
        let pointer = __swift_bridge__$Vec_GridCell$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return GridCellRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<GridCellRef> {
        UnsafePointer<GridCellRef>(OpaquePointer(__swift_bridge__$Vec_GridCell$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_GridCell$len(vecPtr)
    }
}


public class TextAnnotation: TextAnnotationRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TextAnnotation$_free(ptr)
        }
    }
}
public class TextAnnotationRefMut: TextAnnotationRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TextAnnotationRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TextAnnotationRef {
    public func start() -> UInt32 {
        __swift_bridge__$TextAnnotation$start(ptr)
    }

    public func end() -> UInt32 {
        __swift_bridge__$TextAnnotation$end(ptr)
    }

    public func kind() -> AnnotationKind {
        AnnotationKind(ptr: __swift_bridge__$TextAnnotation$kind(ptr))
    }
}
extension TextAnnotation: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TextAnnotation$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TextAnnotation$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TextAnnotation) {
        __swift_bridge__$Vec_TextAnnotation$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TextAnnotation$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TextAnnotation(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextAnnotationRef> {
        let pointer = __swift_bridge__$Vec_TextAnnotation$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextAnnotationRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextAnnotationRefMut> {
        let pointer = __swift_bridge__$Vec_TextAnnotation$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextAnnotationRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TextAnnotationRef> {
        UnsafePointer<TextAnnotationRef>(OpaquePointer(__swift_bridge__$Vec_TextAnnotation$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TextAnnotation$len(vecPtr)
    }
}


public class ExtractionResult: ExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractionResult$_free(ptr)
        }
    }
}
extension ExtractionResult {
    public convenience init<GenericIntoRustString: IntoRustString>(_ content: GenericIntoRustString, _ mime_type: GenericIntoRustString, _ metadata: Metadata, _ extraction_method: Optional<ExtractionMethod>, _ tables: RustVec<Table>, _ detected_languages: Optional<RustVec<GenericIntoRustString>>, _ chunks: Optional<RustVec<Chunk>>, _ images: Optional<RustVec<ExtractedImage>>, _ pages: Optional<RustVec<PageContent>>, _ elements: Optional<RustVec<Element>>, _ djot_content: Optional<DjotContent>, _ ocr_elements: Optional<RustVec<OcrElement>>, _ document: Optional<DocumentStructure>, _ extracted_keywords: Optional<RustVec<Keyword>>, _ quality_score: Optional<Double>, _ processing_warnings: RustVec<ProcessingWarning>, _ annotations: Optional<RustVec<PdfAnnotation>>, _ children: Optional<RustVec<ArchiveEntry>>, _ uris: Optional<RustVec<Uri>>, _ structured_output: Optional<GenericIntoRustString>, _ code_intelligence: Optional<GenericIntoRustString>, _ llm_usage: Optional<RustVec<LlmUsage>>, _ formatted_content: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$ExtractionResult$new({ let rustString = content.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = mime_type.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {metadata.isOwned = false; return metadata.ptr;}(), { if let val = extraction_method { val.isOwned = false; return val.ptr } else { return nil } }(), { let val = tables; val.isOwned = false; return val.ptr }(), { if let val = detected_languages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = chunks { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = images { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = pages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = elements { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = djot_content { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = ocr_elements { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = document { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = extracted_keywords { val.isOwned = false; return val.ptr } else { return nil } }(), quality_score.intoFfiRepr(), { let val = processing_warnings; val.isOwned = false; return val.ptr }(), { if let val = annotations { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = children { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = uris { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(structured_output) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(code_intelligence) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = llm_usage { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(formatted_content) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class ExtractionResultRefMut: ExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractionResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractionResult$content(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractionResult$mime_type(ptr))
    }

    public func metadata() -> Metadata {
        Metadata(ptr: __swift_bridge__$ExtractionResult$metadata(ptr))
    }

    public func extraction_method() -> Optional<ExtractionMethod> {
        { let val = __swift_bridge__$ExtractionResult$extraction_method(ptr); if val != nil { return ExtractionMethod(ptr: val!) } else { return nil } }()
    }

    public func tables() -> RustVec<Table> {
        RustVec(ptr: __swift_bridge__$ExtractionResult$tables(ptr))
    }

    public func detected_languages() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$ExtractionResult$detected_languages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func chunks() -> Optional<RustVec<Chunk>> {
        { let val = __swift_bridge__$ExtractionResult$chunks(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func images() -> Optional<RustVec<ExtractedImage>> {
        { let val = __swift_bridge__$ExtractionResult$images(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func pages() -> Optional<RustVec<PageContent>> {
        { let val = __swift_bridge__$ExtractionResult$pages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func elements() -> Optional<RustVec<Element>> {
        { let val = __swift_bridge__$ExtractionResult$elements(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func djot_content() -> Optional<DjotContent> {
        { let val = __swift_bridge__$ExtractionResult$djot_content(ptr); if val != nil { return DjotContent(ptr: val!) } else { return nil } }()
    }

    public func ocr_elements() -> Optional<RustVec<OcrElement>> {
        { let val = __swift_bridge__$ExtractionResult$ocr_elements(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func document() -> Optional<DocumentStructure> {
        { let val = __swift_bridge__$ExtractionResult$document(ptr); if val != nil { return DocumentStructure(ptr: val!) } else { return nil } }()
    }

    public func extracted_keywords() -> Optional<RustVec<Keyword>> {
        { let val = __swift_bridge__$ExtractionResult$extracted_keywords(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func quality_score() -> Optional<Double> {
        __swift_bridge__$ExtractionResult$quality_score(ptr).intoSwiftRepr()
    }

    public func processing_warnings() -> RustVec<ProcessingWarning> {
        RustVec(ptr: __swift_bridge__$ExtractionResult$processing_warnings(ptr))
    }

    public func annotations() -> Optional<RustVec<PdfAnnotation>> {
        { let val = __swift_bridge__$ExtractionResult$annotations(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func children() -> Optional<RustVec<ArchiveEntry>> {
        { let val = __swift_bridge__$ExtractionResult$children(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func uris() -> Optional<RustVec<Uri>> {
        { let val = __swift_bridge__$ExtractionResult$uris(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func structured_output() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionResult$structured_output(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func code_intelligence() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionResult$code_intelligence(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func llm_usage() -> Optional<RustVec<LlmUsage>> {
        { let val = __swift_bridge__$ExtractionResult$llm_usage(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func formatted_content() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractionResult$formatted_content(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension ExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractionResult) {
        __swift_bridge__$Vec_ExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_ExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractionResultRef> {
        UnsafePointer<ExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_ExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractionResult$len(vecPtr)
    }
}


public class ArchiveEntry: ArchiveEntryRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ArchiveEntry$_free(ptr)
        }
    }
}
public class ArchiveEntryRefMut: ArchiveEntryRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ArchiveEntryRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ArchiveEntryRef {
    public func path() -> RustString {
        RustString(ptr: __swift_bridge__$ArchiveEntry$path(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$ArchiveEntry$mime_type(ptr))
    }

    public func result() -> ExtractionResult {
        ExtractionResult(ptr: __swift_bridge__$ArchiveEntry$result(ptr))
    }
}
extension ArchiveEntry: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ArchiveEntry$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ArchiveEntry$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ArchiveEntry) {
        __swift_bridge__$Vec_ArchiveEntry$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ArchiveEntry$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ArchiveEntry(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ArchiveEntryRef> {
        let pointer = __swift_bridge__$Vec_ArchiveEntry$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ArchiveEntryRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ArchiveEntryRefMut> {
        let pointer = __swift_bridge__$Vec_ArchiveEntry$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ArchiveEntryRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ArchiveEntryRef> {
        UnsafePointer<ArchiveEntryRef>(OpaquePointer(__swift_bridge__$Vec_ArchiveEntry$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ArchiveEntry$len(vecPtr)
    }
}


public class ProcessingWarning: ProcessingWarningRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ProcessingWarning$_free(ptr)
        }
    }
}
public class ProcessingWarningRefMut: ProcessingWarningRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ProcessingWarningRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ProcessingWarningRef {
    public func source() -> RustString {
        RustString(ptr: __swift_bridge__$ProcessingWarning$source(ptr))
    }

    public func message() -> RustString {
        RustString(ptr: __swift_bridge__$ProcessingWarning$message(ptr))
    }
}
extension ProcessingWarning: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ProcessingWarning$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ProcessingWarning$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ProcessingWarning) {
        __swift_bridge__$Vec_ProcessingWarning$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ProcessingWarning$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ProcessingWarning(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ProcessingWarningRef> {
        let pointer = __swift_bridge__$Vec_ProcessingWarning$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ProcessingWarningRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ProcessingWarningRefMut> {
        let pointer = __swift_bridge__$Vec_ProcessingWarning$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ProcessingWarningRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ProcessingWarningRef> {
        UnsafePointer<ProcessingWarningRef>(OpaquePointer(__swift_bridge__$Vec_ProcessingWarning$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ProcessingWarning$len(vecPtr)
    }
}


public class LlmUsage: LlmUsageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LlmUsage$_free(ptr)
        }
    }
}
extension LlmUsage {
    public convenience init<GenericIntoRustString: IntoRustString>(_ model: GenericIntoRustString, _ source: GenericIntoRustString, _ input_tokens: Optional<UInt64>, _ output_tokens: Optional<UInt64>, _ total_tokens: Optional<UInt64>, _ estimated_cost: Optional<Double>, _ finish_reason: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$LlmUsage$new({ let rustString = model.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = source.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), input_tokens.intoFfiRepr(), output_tokens.intoFfiRepr(), total_tokens.intoFfiRepr(), estimated_cost.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(finish_reason) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class LlmUsageRefMut: LlmUsageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LlmUsageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LlmUsageRef {
    public func model() -> RustString {
        RustString(ptr: __swift_bridge__$LlmUsage$model(ptr))
    }

    public func source() -> RustString {
        RustString(ptr: __swift_bridge__$LlmUsage$source(ptr))
    }

    public func input_tokens() -> Optional<UInt64> {
        __swift_bridge__$LlmUsage$input_tokens(ptr).intoSwiftRepr()
    }

    public func output_tokens() -> Optional<UInt64> {
        __swift_bridge__$LlmUsage$output_tokens(ptr).intoSwiftRepr()
    }

    public func total_tokens() -> Optional<UInt64> {
        __swift_bridge__$LlmUsage$total_tokens(ptr).intoSwiftRepr()
    }

    public func estimated_cost() -> Optional<Double> {
        __swift_bridge__$LlmUsage$estimated_cost(ptr).intoSwiftRepr()
    }

    public func finish_reason() -> Optional<RustString> {
        { let val = __swift_bridge__$LlmUsage$finish_reason(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension LlmUsage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LlmUsage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LlmUsage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LlmUsage) {
        __swift_bridge__$Vec_LlmUsage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LlmUsage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LlmUsage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LlmUsageRef> {
        let pointer = __swift_bridge__$Vec_LlmUsage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LlmUsageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LlmUsageRefMut> {
        let pointer = __swift_bridge__$Vec_LlmUsage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LlmUsageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LlmUsageRef> {
        UnsafePointer<LlmUsageRef>(OpaquePointer(__swift_bridge__$Vec_LlmUsage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LlmUsage$len(vecPtr)
    }
}


public class Chunk: ChunkRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Chunk$_free(ptr)
        }
    }
}
public class ChunkRefMut: ChunkRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$Chunk$content(ptr))
    }

    public func chunk_type() -> ChunkType {
        ChunkType(ptr: __swift_bridge__$Chunk$chunk_type(ptr))
    }

    public func embedding() -> Optional<RustVec<Float>> {
        { let val = __swift_bridge__$Chunk$embedding(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func metadata() -> ChunkMetadata {
        ChunkMetadata(ptr: __swift_bridge__$Chunk$metadata(ptr))
    }
}
extension Chunk: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Chunk$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Chunk$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Chunk) {
        __swift_bridge__$Vec_Chunk$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Chunk$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Chunk(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkRef> {
        let pointer = __swift_bridge__$Vec_Chunk$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkRefMut> {
        let pointer = __swift_bridge__$Vec_Chunk$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkRef> {
        UnsafePointer<ChunkRef>(OpaquePointer(__swift_bridge__$Vec_Chunk$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Chunk$len(vecPtr)
    }
}


public class HeadingContext: HeadingContextRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HeadingContext$_free(ptr)
        }
    }
}
public class HeadingContextRefMut: HeadingContextRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HeadingContextRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HeadingContextRef {
    public func headings() -> RustVec<HeadingLevel> {
        RustVec(ptr: __swift_bridge__$HeadingContext$headings(ptr))
    }
}
extension HeadingContext: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HeadingContext$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HeadingContext$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HeadingContext) {
        __swift_bridge__$Vec_HeadingContext$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HeadingContext$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HeadingContext(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeadingContextRef> {
        let pointer = __swift_bridge__$Vec_HeadingContext$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeadingContextRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeadingContextRefMut> {
        let pointer = __swift_bridge__$Vec_HeadingContext$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeadingContextRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HeadingContextRef> {
        UnsafePointer<HeadingContextRef>(OpaquePointer(__swift_bridge__$Vec_HeadingContext$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HeadingContext$len(vecPtr)
    }
}


public class HeadingLevel: HeadingLevelRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HeadingLevel$_free(ptr)
        }
    }
}
public class HeadingLevelRefMut: HeadingLevelRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HeadingLevelRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HeadingLevelRef {
    public func level() -> UInt8 {
        __swift_bridge__$HeadingLevel$level(ptr)
    }

    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$HeadingLevel$text(ptr))
    }
}
extension HeadingLevel: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HeadingLevel$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HeadingLevel$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HeadingLevel) {
        __swift_bridge__$Vec_HeadingLevel$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HeadingLevel$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HeadingLevel(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeadingLevelRef> {
        let pointer = __swift_bridge__$Vec_HeadingLevel$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeadingLevelRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeadingLevelRefMut> {
        let pointer = __swift_bridge__$Vec_HeadingLevel$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeadingLevelRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HeadingLevelRef> {
        UnsafePointer<HeadingLevelRef>(OpaquePointer(__swift_bridge__$Vec_HeadingLevel$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HeadingLevel$len(vecPtr)
    }
}


public class ChunkMetadata: ChunkMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkMetadata$_free(ptr)
        }
    }
}
public class ChunkMetadataRefMut: ChunkMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkMetadataRef {
    public func byte_start() -> UInt {
        __swift_bridge__$ChunkMetadata$byte_start(ptr)
    }

    public func byte_end() -> UInt {
        __swift_bridge__$ChunkMetadata$byte_end(ptr)
    }

    public func token_count() -> Optional<UInt> {
        __swift_bridge__$ChunkMetadata$token_count(ptr).intoSwiftRepr()
    }

    public func chunk_index() -> UInt {
        __swift_bridge__$ChunkMetadata$chunk_index(ptr)
    }

    public func total_chunks() -> UInt {
        __swift_bridge__$ChunkMetadata$total_chunks(ptr)
    }

    public func first_page() -> Optional<UInt> {
        __swift_bridge__$ChunkMetadata$first_page(ptr).intoSwiftRepr()
    }

    public func last_page() -> Optional<UInt> {
        __swift_bridge__$ChunkMetadata$last_page(ptr).intoSwiftRepr()
    }

    public func heading_context() -> Optional<HeadingContext> {
        { let val = __swift_bridge__$ChunkMetadata$heading_context(ptr); if val != nil { return HeadingContext(ptr: val!) } else { return nil } }()
    }
}
extension ChunkMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkMetadata) {
        __swift_bridge__$Vec_ChunkMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkMetadataRef> {
        let pointer = __swift_bridge__$Vec_ChunkMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkMetadataRef> {
        UnsafePointer<ChunkMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ChunkMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkMetadata$len(vecPtr)
    }
}


public class ExtractedImage: ExtractedImageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractedImage$_free(ptr)
        }
    }
}
public class ExtractedImageRefMut: ExtractedImageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractedImageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractedImageRef {
    public func data() -> RustVec<UInt8> {
        RustVec(ptr: __swift_bridge__$ExtractedImage$data(ptr))
    }

    public func format() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractedImage$format(ptr))
    }

    public func image_index() -> UInt {
        __swift_bridge__$ExtractedImage$image_index(ptr)
    }

    public func page_number() -> Optional<UInt> {
        __swift_bridge__$ExtractedImage$page_number(ptr).intoSwiftRepr()
    }

    public func width() -> Optional<UInt32> {
        __swift_bridge__$ExtractedImage$width(ptr).intoSwiftRepr()
    }

    public func height() -> Optional<UInt32> {
        __swift_bridge__$ExtractedImage$height(ptr).intoSwiftRepr()
    }

    public func colorspace() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedImage$colorspace(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func bits_per_component() -> Optional<UInt32> {
        __swift_bridge__$ExtractedImage$bits_per_component(ptr).intoSwiftRepr()
    }

    public func is_mask() -> Bool {
        __swift_bridge__$ExtractedImage$is_mask(ptr)
    }

    public func description() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedImage$description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func ocr_result() -> Optional<ExtractionResult> {
        { let val = __swift_bridge__$ExtractedImage$ocr_result(ptr); if val != nil { return ExtractionResult(ptr: val!) } else { return nil } }()
    }

    public func bounding_box() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedImage$bounding_box(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func source_path() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractedImage$source_path(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func image_kind() -> Optional<ImageKind> {
        { let val = __swift_bridge__$ExtractedImage$image_kind(ptr); if val != nil { return ImageKind(ptr: val!) } else { return nil } }()
    }

    public func kind_confidence() -> Optional<Float> {
        __swift_bridge__$ExtractedImage$kind_confidence(ptr).intoSwiftRepr()
    }

    public func cluster_id() -> Optional<UInt32> {
        __swift_bridge__$ExtractedImage$cluster_id(ptr).intoSwiftRepr()
    }
}
extension ExtractedImage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractedImage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractedImage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractedImage) {
        __swift_bridge__$Vec_ExtractedImage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractedImage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractedImage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractedImageRef> {
        let pointer = __swift_bridge__$Vec_ExtractedImage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractedImageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractedImageRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractedImage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractedImageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractedImageRef> {
        UnsafePointer<ExtractedImageRef>(OpaquePointer(__swift_bridge__$Vec_ExtractedImage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractedImage$len(vecPtr)
    }
}


public class ElementMetadata: ElementMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ElementMetadata$_free(ptr)
        }
    }
}
public class ElementMetadataRefMut: ElementMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ElementMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ElementMetadataRef {
    public func page_number() -> Optional<UInt> {
        __swift_bridge__$ElementMetadata$page_number(ptr).intoSwiftRepr()
    }

    public func filename() -> Optional<RustString> {
        { let val = __swift_bridge__$ElementMetadata$filename(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func coordinates() -> Optional<RustString> {
        { let val = __swift_bridge__$ElementMetadata$coordinates(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func element_index() -> Optional<UInt> {
        __swift_bridge__$ElementMetadata$element_index(ptr).intoSwiftRepr()
    }

    public func additional() -> RustString {
        RustString(ptr: __swift_bridge__$ElementMetadata$additional(ptr))
    }
}
extension ElementMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ElementMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ElementMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ElementMetadata) {
        __swift_bridge__$Vec_ElementMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ElementMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ElementMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementMetadataRef> {
        let pointer = __swift_bridge__$Vec_ElementMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ElementMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ElementMetadataRef> {
        UnsafePointer<ElementMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ElementMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ElementMetadata$len(vecPtr)
    }
}


public class Element: ElementRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Element$_free(ptr)
        }
    }
}
public class ElementRefMut: ElementRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ElementRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ElementRef {
    public func element_id() -> RustString {
        RustString(ptr: __swift_bridge__$Element$element_id(ptr))
    }

    public func element_type() -> ElementType {
        ElementType(ptr: __swift_bridge__$Element$element_type(ptr))
    }

    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$Element$text(ptr))
    }

    public func metadata() -> ElementMetadata {
        ElementMetadata(ptr: __swift_bridge__$Element$metadata(ptr))
    }
}
extension Element: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Element$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Element$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Element) {
        __swift_bridge__$Vec_Element$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Element$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Element(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementRef> {
        let pointer = __swift_bridge__$Vec_Element$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementRefMut> {
        let pointer = __swift_bridge__$Vec_Element$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ElementRef> {
        UnsafePointer<ElementRef>(OpaquePointer(__swift_bridge__$Vec_Element$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Element$len(vecPtr)
    }
}


public class ExcelWorkbook: ExcelWorkbookRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExcelWorkbook$_free(ptr)
        }
    }
}
public class ExcelWorkbookRefMut: ExcelWorkbookRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExcelWorkbookRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExcelWorkbookRef {
    public func sheets() -> RustVec<ExcelSheet> {
        RustVec(ptr: __swift_bridge__$ExcelWorkbook$sheets(ptr))
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$ExcelWorkbook$metadata(ptr))
    }
}
extension ExcelWorkbook: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExcelWorkbook$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExcelWorkbook$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExcelWorkbook) {
        __swift_bridge__$Vec_ExcelWorkbook$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExcelWorkbook$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExcelWorkbook(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelWorkbookRef> {
        let pointer = __swift_bridge__$Vec_ExcelWorkbook$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelWorkbookRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelWorkbookRefMut> {
        let pointer = __swift_bridge__$Vec_ExcelWorkbook$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelWorkbookRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExcelWorkbookRef> {
        UnsafePointer<ExcelWorkbookRef>(OpaquePointer(__swift_bridge__$Vec_ExcelWorkbook$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExcelWorkbook$len(vecPtr)
    }
}


public class ExcelSheet: ExcelSheetRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExcelSheet$_free(ptr)
        }
    }
}
public class ExcelSheetRefMut: ExcelSheetRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExcelSheetRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExcelSheetRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$ExcelSheet$name(ptr))
    }

    public func markdown() -> RustString {
        RustString(ptr: __swift_bridge__$ExcelSheet$markdown(ptr))
    }

    public func row_count() -> UInt {
        __swift_bridge__$ExcelSheet$row_count(ptr)
    }

    public func col_count() -> UInt {
        __swift_bridge__$ExcelSheet$col_count(ptr)
    }

    public func cell_count() -> UInt {
        __swift_bridge__$ExcelSheet$cell_count(ptr)
    }

    public func table_cells() -> RustString {
        RustString(ptr: __swift_bridge__$ExcelSheet$table_cells(ptr))
    }
}
extension ExcelSheet: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExcelSheet$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExcelSheet$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExcelSheet) {
        __swift_bridge__$Vec_ExcelSheet$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExcelSheet$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExcelSheet(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelSheetRef> {
        let pointer = __swift_bridge__$Vec_ExcelSheet$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelSheetRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelSheetRefMut> {
        let pointer = __swift_bridge__$Vec_ExcelSheet$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelSheetRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExcelSheetRef> {
        UnsafePointer<ExcelSheetRef>(OpaquePointer(__swift_bridge__$Vec_ExcelSheet$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExcelSheet$len(vecPtr)
    }
}


public class XmlExtractionResult: XmlExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$XmlExtractionResult$_free(ptr)
        }
    }
}
public class XmlExtractionResultRefMut: XmlExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class XmlExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension XmlExtractionResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$XmlExtractionResult$content(ptr))
    }

    public func element_count() -> UInt {
        __swift_bridge__$XmlExtractionResult$element_count(ptr)
    }

    public func unique_elements() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$XmlExtractionResult$unique_elements(ptr))
    }
}
extension XmlExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_XmlExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_XmlExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: XmlExtractionResult) {
        __swift_bridge__$Vec_XmlExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_XmlExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (XmlExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XmlExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_XmlExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XmlExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XmlExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_XmlExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XmlExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<XmlExtractionResultRef> {
        UnsafePointer<XmlExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_XmlExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_XmlExtractionResult$len(vecPtr)
    }
}


public class TextExtractionResult: TextExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TextExtractionResult$_free(ptr)
        }
    }
}
public class TextExtractionResultRefMut: TextExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TextExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TextExtractionResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$TextExtractionResult$content(ptr))
    }

    public func line_count() -> UInt {
        __swift_bridge__$TextExtractionResult$line_count(ptr)
    }

    public func word_count() -> UInt {
        __swift_bridge__$TextExtractionResult$word_count(ptr)
    }

    public func character_count() -> UInt {
        __swift_bridge__$TextExtractionResult$character_count(ptr)
    }

    public func headers() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextExtractionResult$headers(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func links() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextExtractionResult$links(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func code_blocks() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextExtractionResult$code_blocks(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension TextExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TextExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TextExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TextExtractionResult) {
        __swift_bridge__$Vec_TextExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TextExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TextExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_TextExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_TextExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TextExtractionResultRef> {
        UnsafePointer<TextExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_TextExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TextExtractionResult$len(vecPtr)
    }
}


public class PptxExtractionResult: PptxExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PptxExtractionResult$_free(ptr)
        }
    }
}
public class PptxExtractionResultRefMut: PptxExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PptxExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PptxExtractionResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$PptxExtractionResult$content(ptr))
    }

    public func metadata() -> PptxMetadata {
        PptxMetadata(ptr: __swift_bridge__$PptxExtractionResult$metadata(ptr))
    }

    public func slide_count() -> UInt {
        __swift_bridge__$PptxExtractionResult$slide_count(ptr)
    }

    public func image_count() -> UInt {
        __swift_bridge__$PptxExtractionResult$image_count(ptr)
    }

    public func table_count() -> UInt {
        __swift_bridge__$PptxExtractionResult$table_count(ptr)
    }

    public func images() -> RustVec<ExtractedImage> {
        RustVec(ptr: __swift_bridge__$PptxExtractionResult$images(ptr))
    }

    public func page_structure() -> Optional<PageStructure> {
        { let val = __swift_bridge__$PptxExtractionResult$page_structure(ptr); if val != nil { return PageStructure(ptr: val!) } else { return nil } }()
    }

    public func page_contents() -> Optional<RustVec<PageContent>> {
        { let val = __swift_bridge__$PptxExtractionResult$page_contents(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func document() -> Optional<DocumentStructure> {
        { let val = __swift_bridge__$PptxExtractionResult$document(ptr); if val != nil { return DocumentStructure(ptr: val!) } else { return nil } }()
    }

    public func hyperlinks() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$PptxExtractionResult$hyperlinks(ptr))
    }

    public func office_metadata() -> RustString {
        RustString(ptr: __swift_bridge__$PptxExtractionResult$office_metadata(ptr))
    }
}
extension PptxExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PptxExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PptxExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PptxExtractionResult) {
        __swift_bridge__$Vec_PptxExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PptxExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PptxExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_PptxExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_PptxExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PptxExtractionResultRef> {
        UnsafePointer<PptxExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_PptxExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PptxExtractionResult$len(vecPtr)
    }
}


public class EmailExtractionResult: EmailExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmailExtractionResult$_free(ptr)
        }
    }
}
public class EmailExtractionResultRefMut: EmailExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmailExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmailExtractionResultRef {
    public func subject() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$subject(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func from_email() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$from_email(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func to_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailExtractionResult$to_emails(ptr))
    }

    public func cc_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailExtractionResult$cc_emails(ptr))
    }

    public func bcc_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailExtractionResult$bcc_emails(ptr))
    }

    public func date() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$date(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func message_id() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$message_id(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func plain_text() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$plain_text(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func html_content() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailExtractionResult$html_content(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$EmailExtractionResult$content(ptr))
    }

    public func attachments() -> RustVec<EmailAttachment> {
        RustVec(ptr: __swift_bridge__$EmailExtractionResult$attachments(ptr))
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$EmailExtractionResult$metadata(ptr))
    }
}
extension EmailExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmailExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmailExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmailExtractionResult) {
        __swift_bridge__$Vec_EmailExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmailExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmailExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_EmailExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_EmailExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmailExtractionResultRef> {
        UnsafePointer<EmailExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_EmailExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmailExtractionResult$len(vecPtr)
    }
}


public class EmailAttachment: EmailAttachmentRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmailAttachment$_free(ptr)
        }
    }
}
public class EmailAttachmentRefMut: EmailAttachmentRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmailAttachmentRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmailAttachmentRef {
    public func name() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailAttachment$name(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func filename() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailAttachment$filename(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func mime_type() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailAttachment$mime_type(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func size() -> Optional<UInt> {
        __swift_bridge__$EmailAttachment$size(ptr).intoSwiftRepr()
    }

    public func is_image() -> Bool {
        __swift_bridge__$EmailAttachment$is_image(ptr)
    }

    public func data() -> Optional<RustVec<UInt8>> {
        { let val = __swift_bridge__$EmailAttachment$data(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension EmailAttachment: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmailAttachment$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmailAttachment$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmailAttachment) {
        __swift_bridge__$Vec_EmailAttachment$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmailAttachment$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmailAttachment(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailAttachmentRef> {
        let pointer = __swift_bridge__$Vec_EmailAttachment$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailAttachmentRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailAttachmentRefMut> {
        let pointer = __swift_bridge__$Vec_EmailAttachment$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailAttachmentRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmailAttachmentRef> {
        UnsafePointer<EmailAttachmentRef>(OpaquePointer(__swift_bridge__$Vec_EmailAttachment$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmailAttachment$len(vecPtr)
    }
}


public class OcrExtractionResult: OcrExtractionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrExtractionResult$_free(ptr)
        }
    }
}
public class OcrExtractionResultRefMut: OcrExtractionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrExtractionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrExtractionResultRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$OcrExtractionResult$content(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$OcrExtractionResult$mime_type(ptr))
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$OcrExtractionResult$metadata(ptr))
    }

    public func tables() -> RustVec<OcrTable> {
        RustVec(ptr: __swift_bridge__$OcrExtractionResult$tables(ptr))
    }

    public func ocr_elements() -> Optional<RustVec<OcrElement>> {
        { let val = __swift_bridge__$OcrExtractionResult$ocr_elements(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension OcrExtractionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrExtractionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrExtractionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrExtractionResult) {
        __swift_bridge__$Vec_OcrExtractionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrExtractionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrExtractionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrExtractionResultRef> {
        let pointer = __swift_bridge__$Vec_OcrExtractionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrExtractionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrExtractionResultRefMut> {
        let pointer = __swift_bridge__$Vec_OcrExtractionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrExtractionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrExtractionResultRef> {
        UnsafePointer<OcrExtractionResultRef>(OpaquePointer(__swift_bridge__$Vec_OcrExtractionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrExtractionResult$len(vecPtr)
    }
}


public class OcrTable: OcrTableRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrTable$_free(ptr)
        }
    }
}
public class OcrTableRefMut: OcrTableRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrTableRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrTableRef {
    public func cells() -> RustString {
        RustString(ptr: __swift_bridge__$OcrTable$cells(ptr))
    }

    public func markdown() -> RustString {
        RustString(ptr: __swift_bridge__$OcrTable$markdown(ptr))
    }

    public func page_number() -> UInt {
        __swift_bridge__$OcrTable$page_number(ptr)
    }

    public func bounding_box() -> Optional<OcrTableBoundingBox> {
        { let val = __swift_bridge__$OcrTable$bounding_box(ptr); if val != nil { return OcrTableBoundingBox(ptr: val!) } else { return nil } }()
    }
}
extension OcrTable: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrTable$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrTable$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrTable) {
        __swift_bridge__$Vec_OcrTable$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrTable$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrTable(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrTableRef> {
        let pointer = __swift_bridge__$Vec_OcrTable$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrTableRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrTableRefMut> {
        let pointer = __swift_bridge__$Vec_OcrTable$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrTableRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrTableRef> {
        UnsafePointer<OcrTableRef>(OpaquePointer(__swift_bridge__$Vec_OcrTable$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrTable$len(vecPtr)
    }
}


public class OcrTableBoundingBox: OcrTableBoundingBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrTableBoundingBox$_free(ptr)
        }
    }
}
public class OcrTableBoundingBoxRefMut: OcrTableBoundingBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrTableBoundingBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrTableBoundingBoxRef {
    public func left() -> UInt32 {
        __swift_bridge__$OcrTableBoundingBox$left(ptr)
    }

    public func top() -> UInt32 {
        __swift_bridge__$OcrTableBoundingBox$top(ptr)
    }

    public func right() -> UInt32 {
        __swift_bridge__$OcrTableBoundingBox$right(ptr)
    }

    public func bottom() -> UInt32 {
        __swift_bridge__$OcrTableBoundingBox$bottom(ptr)
    }
}
extension OcrTableBoundingBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrTableBoundingBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrTableBoundingBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrTableBoundingBox) {
        __swift_bridge__$Vec_OcrTableBoundingBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrTableBoundingBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrTableBoundingBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrTableBoundingBoxRef> {
        let pointer = __swift_bridge__$Vec_OcrTableBoundingBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrTableBoundingBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrTableBoundingBoxRefMut> {
        let pointer = __swift_bridge__$Vec_OcrTableBoundingBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrTableBoundingBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrTableBoundingBoxRef> {
        UnsafePointer<OcrTableBoundingBoxRef>(OpaquePointer(__swift_bridge__$Vec_OcrTableBoundingBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrTableBoundingBox$len(vecPtr)
    }
}


public class ImagePreprocessingConfig: ImagePreprocessingConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImagePreprocessingConfig$_free(ptr)
        }
    }
}
extension ImagePreprocessingConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ target_dpi: Int32, _ auto_rotate: Bool, _ deskew: Bool, _ denoise: Bool, _ contrast_enhance: Bool, _ binarization_method: GenericIntoRustString, _ invert_colors: Bool) {
        self.init(ptr: __swift_bridge__$ImagePreprocessingConfig$new(target_dpi, auto_rotate, deskew, denoise, contrast_enhance, { let rustString = binarization_method.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), invert_colors))
    }
}
public class ImagePreprocessingConfigRefMut: ImagePreprocessingConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImagePreprocessingConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImagePreprocessingConfigRef {
    public func target_dpi() -> Int32 {
        __swift_bridge__$ImagePreprocessingConfig$target_dpi(ptr)
    }

    public func auto_rotate() -> Bool {
        __swift_bridge__$ImagePreprocessingConfig$auto_rotate(ptr)
    }

    public func deskew() -> Bool {
        __swift_bridge__$ImagePreprocessingConfig$deskew(ptr)
    }

    public func denoise() -> Bool {
        __swift_bridge__$ImagePreprocessingConfig$denoise(ptr)
    }

    public func contrast_enhance() -> Bool {
        __swift_bridge__$ImagePreprocessingConfig$contrast_enhance(ptr)
    }

    public func binarization_method() -> RustString {
        RustString(ptr: __swift_bridge__$ImagePreprocessingConfig$binarization_method(ptr))
    }

    public func invert_colors() -> Bool {
        __swift_bridge__$ImagePreprocessingConfig$invert_colors(ptr)
    }
}
extension ImagePreprocessingConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImagePreprocessingConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImagePreprocessingConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImagePreprocessingConfig) {
        __swift_bridge__$Vec_ImagePreprocessingConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImagePreprocessingConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImagePreprocessingConfigRef> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImagePreprocessingConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImagePreprocessingConfigRefMut> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImagePreprocessingConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImagePreprocessingConfigRef> {
        UnsafePointer<ImagePreprocessingConfigRef>(OpaquePointer(__swift_bridge__$Vec_ImagePreprocessingConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImagePreprocessingConfig$len(vecPtr)
    }
}


public class TesseractConfig: TesseractConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TesseractConfig$_free(ptr)
        }
    }
}
extension TesseractConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ language: GenericIntoRustString, _ psm: Int32, _ output_format: GenericIntoRustString, _ oem: Int32, _ min_confidence: Double, _ preprocessing: Optional<ImagePreprocessingConfig>, _ enable_table_detection: Bool, _ table_min_confidence: Double, _ table_column_threshold: Int32, _ table_row_threshold_ratio: Double, _ use_cache: Bool, _ classify_use_pre_adapted_templates: Bool, _ language_model_ngram_on: Bool, _ tessedit_dont_blkrej_good_wds: Bool, _ tessedit_dont_rowrej_good_wds: Bool, _ tessedit_enable_dict_correction: Bool, _ tessedit_char_whitelist: GenericIntoRustString, _ tessedit_char_blacklist: GenericIntoRustString, _ tessedit_use_primary_params_model: Bool, _ textord_space_size_is_variable: Bool, _ thresholding_method: Bool) {
        self.init(ptr: __swift_bridge__$TesseractConfig$new({ let rustString = language.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), psm, { let rustString = output_format.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), oem, min_confidence, { if let val = preprocessing { val.isOwned = false; return val.ptr } else { return nil } }(), enable_table_detection, table_min_confidence, table_column_threshold, table_row_threshold_ratio, use_cache, classify_use_pre_adapted_templates, language_model_ngram_on, tessedit_dont_blkrej_good_wds, tessedit_dont_rowrej_good_wds, tessedit_enable_dict_correction, { let rustString = tessedit_char_whitelist.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = tessedit_char_blacklist.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), tessedit_use_primary_params_model, textord_space_size_is_variable, thresholding_method))
    }
}
public class TesseractConfigRefMut: TesseractConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TesseractConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TesseractConfigRef {
    public func language() -> RustString {
        RustString(ptr: __swift_bridge__$TesseractConfig$language(ptr))
    }

    public func psm() -> Int32 {
        __swift_bridge__$TesseractConfig$psm(ptr)
    }

    public func output_format() -> RustString {
        RustString(ptr: __swift_bridge__$TesseractConfig$output_format(ptr))
    }

    public func oem() -> Int32 {
        __swift_bridge__$TesseractConfig$oem(ptr)
    }

    public func min_confidence() -> Double {
        __swift_bridge__$TesseractConfig$min_confidence(ptr)
    }

    public func preprocessing() -> Optional<ImagePreprocessingConfig> {
        { let val = __swift_bridge__$TesseractConfig$preprocessing(ptr); if val != nil { return ImagePreprocessingConfig(ptr: val!) } else { return nil } }()
    }

    public func enable_table_detection() -> Bool {
        __swift_bridge__$TesseractConfig$enable_table_detection(ptr)
    }

    public func table_min_confidence() -> Double {
        __swift_bridge__$TesseractConfig$table_min_confidence(ptr)
    }

    public func table_column_threshold() -> Int32 {
        __swift_bridge__$TesseractConfig$table_column_threshold(ptr)
    }

    public func table_row_threshold_ratio() -> Double {
        __swift_bridge__$TesseractConfig$table_row_threshold_ratio(ptr)
    }

    public func use_cache() -> Bool {
        __swift_bridge__$TesseractConfig$use_cache(ptr)
    }

    public func classify_use_pre_adapted_templates() -> Bool {
        __swift_bridge__$TesseractConfig$classify_use_pre_adapted_templates(ptr)
    }

    public func language_model_ngram_on() -> Bool {
        __swift_bridge__$TesseractConfig$language_model_ngram_on(ptr)
    }

    public func tessedit_dont_blkrej_good_wds() -> Bool {
        __swift_bridge__$TesseractConfig$tessedit_dont_blkrej_good_wds(ptr)
    }

    public func tessedit_dont_rowrej_good_wds() -> Bool {
        __swift_bridge__$TesseractConfig$tessedit_dont_rowrej_good_wds(ptr)
    }

    public func tessedit_enable_dict_correction() -> Bool {
        __swift_bridge__$TesseractConfig$tessedit_enable_dict_correction(ptr)
    }

    public func tessedit_char_whitelist() -> RustString {
        RustString(ptr: __swift_bridge__$TesseractConfig$tessedit_char_whitelist(ptr))
    }

    public func tessedit_char_blacklist() -> RustString {
        RustString(ptr: __swift_bridge__$TesseractConfig$tessedit_char_blacklist(ptr))
    }

    public func tessedit_use_primary_params_model() -> Bool {
        __swift_bridge__$TesseractConfig$tessedit_use_primary_params_model(ptr)
    }

    public func textord_space_size_is_variable() -> Bool {
        __swift_bridge__$TesseractConfig$textord_space_size_is_variable(ptr)
    }

    public func thresholding_method() -> Bool {
        __swift_bridge__$TesseractConfig$thresholding_method(ptr)
    }
}
extension TesseractConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TesseractConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TesseractConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TesseractConfig) {
        __swift_bridge__$Vec_TesseractConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TesseractConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TesseractConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TesseractConfigRef> {
        let pointer = __swift_bridge__$Vec_TesseractConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TesseractConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TesseractConfigRefMut> {
        let pointer = __swift_bridge__$Vec_TesseractConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TesseractConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TesseractConfigRef> {
        UnsafePointer<TesseractConfigRef>(OpaquePointer(__swift_bridge__$Vec_TesseractConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TesseractConfig$len(vecPtr)
    }
}


public class ImagePreprocessingMetadata: ImagePreprocessingMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImagePreprocessingMetadata$_free(ptr)
        }
    }
}
public class ImagePreprocessingMetadataRefMut: ImagePreprocessingMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImagePreprocessingMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImagePreprocessingMetadataRef {
    public func original_dimensions() -> RustVec<UInt> {
        RustVec(ptr: __swift_bridge__$ImagePreprocessingMetadata$original_dimensions(ptr))
    }

    public func original_dpi() -> RustVec<Double> {
        RustVec(ptr: __swift_bridge__$ImagePreprocessingMetadata$original_dpi(ptr))
    }

    public func target_dpi() -> Int32 {
        __swift_bridge__$ImagePreprocessingMetadata$target_dpi(ptr)
    }

    public func scale_factor() -> Double {
        __swift_bridge__$ImagePreprocessingMetadata$scale_factor(ptr)
    }

    public func auto_adjusted() -> Bool {
        __swift_bridge__$ImagePreprocessingMetadata$auto_adjusted(ptr)
    }

    public func final_dpi() -> Int32 {
        __swift_bridge__$ImagePreprocessingMetadata$final_dpi(ptr)
    }

    public func new_dimensions() -> Optional<RustVec<UInt>> {
        { let val = __swift_bridge__$ImagePreprocessingMetadata$new_dimensions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func resample_method() -> RustString {
        RustString(ptr: __swift_bridge__$ImagePreprocessingMetadata$resample_method(ptr))
    }

    public func dimension_clamped() -> Bool {
        __swift_bridge__$ImagePreprocessingMetadata$dimension_clamped(ptr)
    }

    public func calculated_dpi() -> Optional<Int32> {
        __swift_bridge__$ImagePreprocessingMetadata$calculated_dpi(ptr).intoSwiftRepr()
    }

    public func skipped_resize() -> Bool {
        __swift_bridge__$ImagePreprocessingMetadata$skipped_resize(ptr)
    }

    public func resize_error() -> Optional<RustString> {
        { let val = __swift_bridge__$ImagePreprocessingMetadata$resize_error(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension ImagePreprocessingMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImagePreprocessingMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImagePreprocessingMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImagePreprocessingMetadata) {
        __swift_bridge__$Vec_ImagePreprocessingMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImagePreprocessingMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImagePreprocessingMetadataRef> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImagePreprocessingMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImagePreprocessingMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ImagePreprocessingMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImagePreprocessingMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImagePreprocessingMetadataRef> {
        UnsafePointer<ImagePreprocessingMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ImagePreprocessingMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImagePreprocessingMetadata$len(vecPtr)
    }
}


public class Metadata: MetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Metadata$_free(ptr)
        }
    }
}
extension Metadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ title: Optional<GenericIntoRustString>, _ subject: Optional<GenericIntoRustString>, _ authors: Optional<RustVec<GenericIntoRustString>>, _ keywords: Optional<RustVec<GenericIntoRustString>>, _ language: Optional<GenericIntoRustString>, _ created_at: Optional<GenericIntoRustString>, _ modified_at: Optional<GenericIntoRustString>, _ created_by: Optional<GenericIntoRustString>, _ modified_by: Optional<GenericIntoRustString>, _ pages: Optional<PageStructure>, _ format: Optional<FormatMetadata>, _ image_preprocessing: Optional<ImagePreprocessingMetadata>, _ json_schema: Optional<GenericIntoRustString>, _ error: Optional<ErrorMetadata>, _ extraction_duration_ms: Optional<UInt64>, _ category: Optional<GenericIntoRustString>, _ tags: Optional<RustVec<GenericIntoRustString>>, _ document_version: Optional<GenericIntoRustString>, _ abstract_text: Optional<GenericIntoRustString>, _ output_format: Optional<GenericIntoRustString>, _ additional: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$Metadata$new({ if let rustString = optionalStringIntoRustString(title) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(subject) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = authors { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = keywords { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(language) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(created_at) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(modified_at) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(created_by) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(modified_by) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = pages { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = format { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = image_preprocessing { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(json_schema) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = error { val.isOwned = false; return val.ptr } else { return nil } }(), extraction_duration_ms.intoFfiRepr(), { if let rustString = optionalStringIntoRustString(category) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = tags { val.isOwned = false; return val.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(document_version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(abstract_text) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(output_format) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let rustString = additional.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class MetadataRefMut: MetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class MetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension MetadataRef {
    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func subject() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$subject(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func authors() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$Metadata$authors(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$Metadata$keywords(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func created_at() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$created_at(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func modified_at() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$modified_at(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func created_by() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$created_by(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func modified_by() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$modified_by(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func pages() -> Optional<PageStructure> {
        { let val = __swift_bridge__$Metadata$pages(ptr); if val != nil { return PageStructure(ptr: val!) } else { return nil } }()
    }

    public func format() -> Optional<FormatMetadata> {
        { let val = __swift_bridge__$Metadata$format(ptr); if val != nil { return FormatMetadata(ptr: val!) } else { return nil } }()
    }

    public func image_preprocessing() -> Optional<ImagePreprocessingMetadata> {
        { let val = __swift_bridge__$Metadata$image_preprocessing(ptr); if val != nil { return ImagePreprocessingMetadata(ptr: val!) } else { return nil } }()
    }

    public func json_schema() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$json_schema(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func error() -> Optional<ErrorMetadata> {
        { let val = __swift_bridge__$Metadata$error(ptr); if val != nil { return ErrorMetadata(ptr: val!) } else { return nil } }()
    }

    public func extraction_duration_ms() -> Optional<UInt64> {
        __swift_bridge__$Metadata$extraction_duration_ms(ptr).intoSwiftRepr()
    }

    public func category() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$category(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func tags() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$Metadata$tags(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func document_version() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$document_version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func abstract_text() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$abstract_text(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func output_format() -> Optional<RustString> {
        { let val = __swift_bridge__$Metadata$output_format(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func additional() -> RustString {
        RustString(ptr: __swift_bridge__$Metadata$additional(ptr))
    }
}
extension Metadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Metadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Metadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Metadata) {
        __swift_bridge__$Vec_Metadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Metadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Metadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<MetadataRef> {
        let pointer = __swift_bridge__$Vec_Metadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return MetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<MetadataRefMut> {
        let pointer = __swift_bridge__$Vec_Metadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return MetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<MetadataRef> {
        UnsafePointer<MetadataRef>(OpaquePointer(__swift_bridge__$Vec_Metadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Metadata$len(vecPtr)
    }
}


public class ExcelMetadata: ExcelMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExcelMetadata$_free(ptr)
        }
    }
}
extension ExcelMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ sheet_count: Optional<UInt>, _ sheet_names: Optional<RustVec<GenericIntoRustString>>) {
        self.init(ptr: __swift_bridge__$ExcelMetadata$new(sheet_count.intoFfiRepr(), { if let val = sheet_names { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class ExcelMetadataRefMut: ExcelMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExcelMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExcelMetadataRef {
    public func sheet_count() -> Optional<UInt> {
        __swift_bridge__$ExcelMetadata$sheet_count(ptr).intoSwiftRepr()
    }

    public func sheet_names() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$ExcelMetadata$sheet_names(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension ExcelMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExcelMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExcelMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExcelMetadata) {
        __swift_bridge__$Vec_ExcelMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExcelMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExcelMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelMetadataRef> {
        let pointer = __swift_bridge__$Vec_ExcelMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExcelMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ExcelMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExcelMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExcelMetadataRef> {
        UnsafePointer<ExcelMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ExcelMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExcelMetadata$len(vecPtr)
    }
}


public class EmailMetadata: EmailMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmailMetadata$_free(ptr)
        }
    }
}
extension EmailMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ from_email: Optional<GenericIntoRustString>, _ from_name: Optional<GenericIntoRustString>, _ to_emails: RustVec<GenericIntoRustString>, _ cc_emails: RustVec<GenericIntoRustString>, _ bcc_emails: RustVec<GenericIntoRustString>, _ message_id: Optional<GenericIntoRustString>, _ attachments: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$EmailMetadata$new({ if let rustString = optionalStringIntoRustString(from_email) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(from_name) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = to_emails; val.isOwned = false; return val.ptr }(), { let val = cc_emails; val.isOwned = false; return val.ptr }(), { let val = bcc_emails; val.isOwned = false; return val.ptr }(), { if let rustString = optionalStringIntoRustString(message_id) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = attachments; val.isOwned = false; return val.ptr }()))
    }
}
public class EmailMetadataRefMut: EmailMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmailMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmailMetadataRef {
    public func from_email() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailMetadata$from_email(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func from_name() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailMetadata$from_name(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func to_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailMetadata$to_emails(ptr))
    }

    public func cc_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailMetadata$cc_emails(ptr))
    }

    public func bcc_emails() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailMetadata$bcc_emails(ptr))
    }

    public func message_id() -> Optional<RustString> {
        { let val = __swift_bridge__$EmailMetadata$message_id(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func attachments() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmailMetadata$attachments(ptr))
    }
}
extension EmailMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmailMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmailMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmailMetadata) {
        __swift_bridge__$Vec_EmailMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmailMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmailMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailMetadataRef> {
        let pointer = __swift_bridge__$Vec_EmailMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmailMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_EmailMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmailMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmailMetadataRef> {
        UnsafePointer<EmailMetadataRef>(OpaquePointer(__swift_bridge__$Vec_EmailMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmailMetadata$len(vecPtr)
    }
}


public class ArchiveMetadata: ArchiveMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ArchiveMetadata$_free(ptr)
        }
    }
}
extension ArchiveMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ format: GenericIntoRustString, _ file_count: UInt, _ file_list: RustVec<GenericIntoRustString>, _ total_size: UInt, _ compressed_size: Optional<UInt>) {
        self.init(ptr: __swift_bridge__$ArchiveMetadata$new({ let rustString = format.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), file_count, { let val = file_list; val.isOwned = false; return val.ptr }(), total_size, compressed_size.intoFfiRepr()))
    }
}
public class ArchiveMetadataRefMut: ArchiveMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ArchiveMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ArchiveMetadataRef {
    public func format() -> RustString {
        RustString(ptr: __swift_bridge__$ArchiveMetadata$format(ptr))
    }

    public func file_count() -> UInt {
        __swift_bridge__$ArchiveMetadata$file_count(ptr)
    }

    public func file_list() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$ArchiveMetadata$file_list(ptr))
    }

    public func total_size() -> UInt {
        __swift_bridge__$ArchiveMetadata$total_size(ptr)
    }

    public func compressed_size() -> Optional<UInt> {
        __swift_bridge__$ArchiveMetadata$compressed_size(ptr).intoSwiftRepr()
    }
}
extension ArchiveMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ArchiveMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ArchiveMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ArchiveMetadata) {
        __swift_bridge__$Vec_ArchiveMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ArchiveMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ArchiveMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ArchiveMetadataRef> {
        let pointer = __swift_bridge__$Vec_ArchiveMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ArchiveMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ArchiveMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ArchiveMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ArchiveMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ArchiveMetadataRef> {
        UnsafePointer<ArchiveMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ArchiveMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ArchiveMetadata$len(vecPtr)
    }
}


public class ImageMetadata: ImageMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageMetadata$_free(ptr)
        }
    }
}
extension ImageMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ width: UInt32, _ height: UInt32, _ format: GenericIntoRustString, _ exif: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$ImageMetadata$new(width, height, { let rustString = format.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = exif.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class ImageMetadataRefMut: ImageMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageMetadataRef {
    public func width() -> UInt32 {
        __swift_bridge__$ImageMetadata$width(ptr)
    }

    public func height() -> UInt32 {
        __swift_bridge__$ImageMetadata$height(ptr)
    }

    public func format() -> RustString {
        RustString(ptr: __swift_bridge__$ImageMetadata$format(ptr))
    }

    public func exif() -> RustString {
        RustString(ptr: __swift_bridge__$ImageMetadata$exif(ptr))
    }
}
extension ImageMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageMetadata) {
        __swift_bridge__$Vec_ImageMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageMetadataRef> {
        let pointer = __swift_bridge__$Vec_ImageMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ImageMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageMetadataRef> {
        UnsafePointer<ImageMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ImageMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageMetadata$len(vecPtr)
    }
}


public class XmlMetadata: XmlMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$XmlMetadata$_free(ptr)
        }
    }
}
extension XmlMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ element_count: UInt, _ unique_elements: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$XmlMetadata$new(element_count, { let val = unique_elements; val.isOwned = false; return val.ptr }()))
    }
}
public class XmlMetadataRefMut: XmlMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class XmlMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension XmlMetadataRef {
    public func element_count() -> UInt {
        __swift_bridge__$XmlMetadata$element_count(ptr)
    }

    public func unique_elements() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$XmlMetadata$unique_elements(ptr))
    }
}
extension XmlMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_XmlMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_XmlMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: XmlMetadata) {
        __swift_bridge__$Vec_XmlMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_XmlMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (XmlMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XmlMetadataRef> {
        let pointer = __swift_bridge__$Vec_XmlMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XmlMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<XmlMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_XmlMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return XmlMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<XmlMetadataRef> {
        UnsafePointer<XmlMetadataRef>(OpaquePointer(__swift_bridge__$Vec_XmlMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_XmlMetadata$len(vecPtr)
    }
}


public class TextMetadata: TextMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TextMetadata$_free(ptr)
        }
    }
}
extension TextMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ line_count: UInt, _ word_count: UInt, _ character_count: UInt, _ headers: Optional<RustVec<GenericIntoRustString>>, _ links: Optional<RustVec<GenericIntoRustString>>, _ code_blocks: Optional<RustVec<GenericIntoRustString>>) {
        self.init(ptr: __swift_bridge__$TextMetadata$new(line_count, word_count, character_count, { if let val = headers { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = links { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = code_blocks { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class TextMetadataRefMut: TextMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TextMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TextMetadataRef {
    public func line_count() -> UInt {
        __swift_bridge__$TextMetadata$line_count(ptr)
    }

    public func word_count() -> UInt {
        __swift_bridge__$TextMetadata$word_count(ptr)
    }

    public func character_count() -> UInt {
        __swift_bridge__$TextMetadata$character_count(ptr)
    }

    public func headers() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextMetadata$headers(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func links() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextMetadata$links(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func code_blocks() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$TextMetadata$code_blocks(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension TextMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TextMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TextMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TextMetadata) {
        __swift_bridge__$Vec_TextMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TextMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TextMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextMetadataRef> {
        let pointer = __swift_bridge__$Vec_TextMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_TextMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TextMetadataRef> {
        UnsafePointer<TextMetadataRef>(OpaquePointer(__swift_bridge__$Vec_TextMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TextMetadata$len(vecPtr)
    }
}


public class HeaderMetadata: HeaderMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HeaderMetadata$_free(ptr)
        }
    }
}
public class HeaderMetadataRefMut: HeaderMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HeaderMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HeaderMetadataRef {
    public func level() -> UInt8 {
        __swift_bridge__$HeaderMetadata$level(ptr)
    }

    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$HeaderMetadata$text(ptr))
    }

    public func id() -> Optional<RustString> {
        { let val = __swift_bridge__$HeaderMetadata$id(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func depth() -> UInt {
        __swift_bridge__$HeaderMetadata$depth(ptr)
    }

    public func html_offset() -> UInt {
        __swift_bridge__$HeaderMetadata$html_offset(ptr)
    }
}
extension HeaderMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HeaderMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HeaderMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HeaderMetadata) {
        __swift_bridge__$Vec_HeaderMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HeaderMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HeaderMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeaderMetadataRef> {
        let pointer = __swift_bridge__$Vec_HeaderMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeaderMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HeaderMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_HeaderMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HeaderMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HeaderMetadataRef> {
        UnsafePointer<HeaderMetadataRef>(OpaquePointer(__swift_bridge__$Vec_HeaderMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HeaderMetadata$len(vecPtr)
    }
}


public class LinkMetadata: LinkMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LinkMetadata$_free(ptr)
        }
    }
}
public class LinkMetadataRefMut: LinkMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LinkMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LinkMetadataRef {
    public func href() -> RustString {
        RustString(ptr: __swift_bridge__$LinkMetadata$href(ptr))
    }

    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$LinkMetadata$text(ptr))
    }

    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$LinkMetadata$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func link_type() -> LinkType {
        LinkType(ptr: __swift_bridge__$LinkMetadata$link_type(ptr))
    }

    public func rel() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$LinkMetadata$rel(ptr))
    }

    public func attributes() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$LinkMetadata$attributes(ptr))
    }
}
extension LinkMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LinkMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LinkMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LinkMetadata) {
        __swift_bridge__$Vec_LinkMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LinkMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LinkMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LinkMetadataRef> {
        let pointer = __swift_bridge__$Vec_LinkMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LinkMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LinkMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_LinkMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LinkMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LinkMetadataRef> {
        UnsafePointer<LinkMetadataRef>(OpaquePointer(__swift_bridge__$Vec_LinkMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LinkMetadata$len(vecPtr)
    }
}


public class ImageMetadataType: ImageMetadataTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageMetadataType$_free(ptr)
        }
    }
}
public class ImageMetadataTypeRefMut: ImageMetadataTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageMetadataTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageMetadataTypeRef {
    public func src() -> RustString {
        RustString(ptr: __swift_bridge__$ImageMetadataType$src(ptr))
    }

    public func alt() -> Optional<RustString> {
        { let val = __swift_bridge__$ImageMetadataType$alt(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$ImageMetadataType$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func dimensions() -> Optional<RustVec<UInt32>> {
        { let val = __swift_bridge__$ImageMetadataType$dimensions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func image_type() -> ImageType {
        ImageType(ptr: __swift_bridge__$ImageMetadataType$image_type(ptr))
    }

    public func attributes() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$ImageMetadataType$attributes(ptr))
    }
}
extension ImageMetadataType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageMetadataType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageMetadataType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageMetadataType) {
        __swift_bridge__$Vec_ImageMetadataType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageMetadataType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageMetadataType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageMetadataTypeRef> {
        let pointer = __swift_bridge__$Vec_ImageMetadataType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageMetadataTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageMetadataTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ImageMetadataType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageMetadataTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageMetadataTypeRef> {
        UnsafePointer<ImageMetadataTypeRef>(OpaquePointer(__swift_bridge__$Vec_ImageMetadataType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageMetadataType$len(vecPtr)
    }
}


public class StructuredData: StructuredDataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StructuredData$_free(ptr)
        }
    }
}
public class StructuredDataRefMut: StructuredDataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StructuredDataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StructuredDataRef {
    public func data_type() -> StructuredDataType {
        StructuredDataType(ptr: __swift_bridge__$StructuredData$data_type(ptr))
    }

    public func raw_json() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredData$raw_json(ptr))
    }

    public func schema_type() -> Optional<RustString> {
        { let val = __swift_bridge__$StructuredData$schema_type(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension StructuredData: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StructuredData$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StructuredData$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StructuredData) {
        __swift_bridge__$Vec_StructuredData$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StructuredData$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StructuredData(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataRef> {
        let pointer = __swift_bridge__$Vec_StructuredData$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataRefMut> {
        let pointer = __swift_bridge__$Vec_StructuredData$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StructuredDataRef> {
        UnsafePointer<StructuredDataRef>(OpaquePointer(__swift_bridge__$Vec_StructuredData$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StructuredData$len(vecPtr)
    }
}


public class HtmlMetadata: HtmlMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HtmlMetadata$_free(ptr)
        }
    }
}
extension HtmlMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ title: Optional<GenericIntoRustString>, _ description: Optional<GenericIntoRustString>, _ keywords: RustVec<GenericIntoRustString>, _ author: Optional<GenericIntoRustString>, _ canonical_url: Optional<GenericIntoRustString>, _ base_href: Optional<GenericIntoRustString>, _ language: Optional<GenericIntoRustString>, _ text_direction: Optional<TextDirection>, _ open_graph: GenericIntoRustString, _ twitter_card: GenericIntoRustString, _ meta_tags: GenericIntoRustString, _ headers: RustVec<HeaderMetadata>, _ links: RustVec<LinkMetadata>, _ images: RustVec<ImageMetadataType>, _ structured_data: RustVec<StructuredData>) {
        self.init(ptr: __swift_bridge__$HtmlMetadata$new({ if let rustString = optionalStringIntoRustString(title) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(description) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = keywords; val.isOwned = false; return val.ptr }(), { if let rustString = optionalStringIntoRustString(author) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(canonical_url) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(base_href) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(language) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = text_direction { val.isOwned = false; return val.ptr } else { return nil } }(), { let rustString = open_graph.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = twitter_card.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = meta_tags.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = headers; val.isOwned = false; return val.ptr }(), { let val = links; val.isOwned = false; return val.ptr }(), { let val = images; val.isOwned = false; return val.ptr }(), { let val = structured_data; val.isOwned = false; return val.ptr }()))
    }
}
public class HtmlMetadataRefMut: HtmlMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HtmlMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HtmlMetadataRef {
    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func description() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func keywords() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$HtmlMetadata$keywords(ptr))
    }

    public func author() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$author(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func canonical_url() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$canonical_url(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func base_href() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$base_href(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$HtmlMetadata$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func text_direction() -> Optional<TextDirection> {
        { let val = __swift_bridge__$HtmlMetadata$text_direction(ptr); if val != nil { return TextDirection(ptr: val!) } else { return nil } }()
    }

    public func open_graph() -> RustString {
        RustString(ptr: __swift_bridge__$HtmlMetadata$open_graph(ptr))
    }

    public func twitter_card() -> RustString {
        RustString(ptr: __swift_bridge__$HtmlMetadata$twitter_card(ptr))
    }

    public func meta_tags() -> RustString {
        RustString(ptr: __swift_bridge__$HtmlMetadata$meta_tags(ptr))
    }

    public func headers() -> RustVec<HeaderMetadata> {
        RustVec(ptr: __swift_bridge__$HtmlMetadata$headers(ptr))
    }

    public func links() -> RustVec<LinkMetadata> {
        RustVec(ptr: __swift_bridge__$HtmlMetadata$links(ptr))
    }

    public func images() -> RustVec<ImageMetadataType> {
        RustVec(ptr: __swift_bridge__$HtmlMetadata$images(ptr))
    }

    public func structured_data() -> RustVec<StructuredData> {
        RustVec(ptr: __swift_bridge__$HtmlMetadata$structured_data(ptr))
    }
}
extension HtmlMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HtmlMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HtmlMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HtmlMetadata) {
        __swift_bridge__$Vec_HtmlMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HtmlMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HtmlMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlMetadataRef> {
        let pointer = __swift_bridge__$Vec_HtmlMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_HtmlMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HtmlMetadataRef> {
        UnsafePointer<HtmlMetadataRef>(OpaquePointer(__swift_bridge__$Vec_HtmlMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HtmlMetadata$len(vecPtr)
    }
}


public class OcrMetadata: OcrMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrMetadata$_free(ptr)
        }
    }
}
extension OcrMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ language: GenericIntoRustString, _ psm: Int32, _ output_format: GenericIntoRustString, _ table_count: UInt, _ table_rows: Optional<UInt>, _ table_cols: Optional<UInt>) {
        self.init(ptr: __swift_bridge__$OcrMetadata$new({ let rustString = language.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), psm, { let rustString = output_format.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), table_count, table_rows.intoFfiRepr(), table_cols.intoFfiRepr()))
    }
}
public class OcrMetadataRefMut: OcrMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrMetadataRef {
    public func language() -> RustString {
        RustString(ptr: __swift_bridge__$OcrMetadata$language(ptr))
    }

    public func psm() -> Int32 {
        __swift_bridge__$OcrMetadata$psm(ptr)
    }

    public func output_format() -> RustString {
        RustString(ptr: __swift_bridge__$OcrMetadata$output_format(ptr))
    }

    public func table_count() -> UInt {
        __swift_bridge__$OcrMetadata$table_count(ptr)
    }

    public func table_rows() -> Optional<UInt> {
        __swift_bridge__$OcrMetadata$table_rows(ptr).intoSwiftRepr()
    }

    public func table_cols() -> Optional<UInt> {
        __swift_bridge__$OcrMetadata$table_cols(ptr).intoSwiftRepr()
    }
}
extension OcrMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrMetadata) {
        __swift_bridge__$Vec_OcrMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrMetadataRef> {
        let pointer = __swift_bridge__$Vec_OcrMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_OcrMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrMetadataRef> {
        UnsafePointer<OcrMetadataRef>(OpaquePointer(__swift_bridge__$Vec_OcrMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrMetadata$len(vecPtr)
    }
}


public class ErrorMetadata: ErrorMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ErrorMetadata$_free(ptr)
        }
    }
}
public class ErrorMetadataRefMut: ErrorMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ErrorMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ErrorMetadataRef {
    public func error_type() -> RustString {
        RustString(ptr: __swift_bridge__$ErrorMetadata$error_type(ptr))
    }

    public func message() -> RustString {
        RustString(ptr: __swift_bridge__$ErrorMetadata$message(ptr))
    }
}
extension ErrorMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ErrorMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ErrorMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ErrorMetadata) {
        __swift_bridge__$Vec_ErrorMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ErrorMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ErrorMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ErrorMetadataRef> {
        let pointer = __swift_bridge__$Vec_ErrorMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ErrorMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ErrorMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_ErrorMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ErrorMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ErrorMetadataRef> {
        UnsafePointer<ErrorMetadataRef>(OpaquePointer(__swift_bridge__$Vec_ErrorMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ErrorMetadata$len(vecPtr)
    }
}


public class PptxMetadata: PptxMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PptxMetadata$_free(ptr)
        }
    }
}
extension PptxMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ slide_count: UInt, _ slide_names: RustVec<GenericIntoRustString>, _ image_count: Optional<UInt>, _ table_count: Optional<UInt>) {
        self.init(ptr: __swift_bridge__$PptxMetadata$new(slide_count, { let val = slide_names; val.isOwned = false; return val.ptr }(), image_count.intoFfiRepr(), table_count.intoFfiRepr()))
    }
}
public class PptxMetadataRefMut: PptxMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PptxMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PptxMetadataRef {
    public func slide_count() -> UInt {
        __swift_bridge__$PptxMetadata$slide_count(ptr)
    }

    public func slide_names() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$PptxMetadata$slide_names(ptr))
    }

    public func image_count() -> Optional<UInt> {
        __swift_bridge__$PptxMetadata$image_count(ptr).intoSwiftRepr()
    }

    public func table_count() -> Optional<UInt> {
        __swift_bridge__$PptxMetadata$table_count(ptr).intoSwiftRepr()
    }
}
extension PptxMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PptxMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PptxMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PptxMetadata) {
        __swift_bridge__$Vec_PptxMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PptxMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PptxMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxMetadataRef> {
        let pointer = __swift_bridge__$Vec_PptxMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PptxMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_PptxMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PptxMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PptxMetadataRef> {
        UnsafePointer<PptxMetadataRef>(OpaquePointer(__swift_bridge__$Vec_PptxMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PptxMetadata$len(vecPtr)
    }
}


public class DocxMetadata: DocxMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DocxMetadata$_free(ptr)
        }
    }
}
extension DocxMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ core_properties: Optional<CoreProperties>, _ app_properties: Optional<DocxAppProperties>, _ custom_properties: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$DocxMetadata$new({ if let val = core_properties { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = app_properties { val.isOwned = false; return val.ptr } else { return nil } }(), { let rustString = custom_properties.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class DocxMetadataRefMut: DocxMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DocxMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DocxMetadataRef {
    public func core_properties() -> Optional<CoreProperties> {
        { let val = __swift_bridge__$DocxMetadata$core_properties(ptr); if val != nil { return CoreProperties(ptr: val!) } else { return nil } }()
    }

    public func app_properties() -> Optional<DocxAppProperties> {
        { let val = __swift_bridge__$DocxMetadata$app_properties(ptr); if val != nil { return DocxAppProperties(ptr: val!) } else { return nil } }()
    }

    public func custom_properties() -> RustString {
        RustString(ptr: __swift_bridge__$DocxMetadata$custom_properties(ptr))
    }
}
extension DocxMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DocxMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DocxMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DocxMetadata) {
        __swift_bridge__$Vec_DocxMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DocxMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DocxMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocxMetadataRef> {
        let pointer = __swift_bridge__$Vec_DocxMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocxMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DocxMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_DocxMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DocxMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DocxMetadataRef> {
        UnsafePointer<DocxMetadataRef>(OpaquePointer(__swift_bridge__$Vec_DocxMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DocxMetadata$len(vecPtr)
    }
}


public class CsvMetadata: CsvMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CsvMetadata$_free(ptr)
        }
    }
}
extension CsvMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ row_count: UInt, _ column_count: UInt, _ delimiter: Optional<GenericIntoRustString>, _ has_header: Bool, _ column_types: Optional<RustVec<GenericIntoRustString>>) {
        self.init(ptr: __swift_bridge__$CsvMetadata$new(row_count, column_count, { if let rustString = optionalStringIntoRustString(delimiter) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), has_header, { if let val = column_types { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class CsvMetadataRefMut: CsvMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CsvMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CsvMetadataRef {
    public func row_count() -> UInt {
        __swift_bridge__$CsvMetadata$row_count(ptr)
    }

    public func column_count() -> UInt {
        __swift_bridge__$CsvMetadata$column_count(ptr)
    }

    public func delimiter() -> Optional<RustString> {
        { let val = __swift_bridge__$CsvMetadata$delimiter(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func has_header() -> Bool {
        __swift_bridge__$CsvMetadata$has_header(ptr)
    }

    public func column_types() -> Optional<RustVec<RustString>> {
        { let val = __swift_bridge__$CsvMetadata$column_types(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension CsvMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CsvMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CsvMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CsvMetadata) {
        __swift_bridge__$Vec_CsvMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CsvMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CsvMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CsvMetadataRef> {
        let pointer = __swift_bridge__$Vec_CsvMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CsvMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CsvMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_CsvMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CsvMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CsvMetadataRef> {
        UnsafePointer<CsvMetadataRef>(OpaquePointer(__swift_bridge__$Vec_CsvMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CsvMetadata$len(vecPtr)
    }
}


public class BibtexMetadata: BibtexMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$BibtexMetadata$_free(ptr)
        }
    }
}
extension BibtexMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ entry_count: UInt, _ citation_keys: RustVec<GenericIntoRustString>, _ authors: RustVec<GenericIntoRustString>, _ year_range: Optional<YearRange>, _ entry_types: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$BibtexMetadata$new(entry_count, { let val = citation_keys; val.isOwned = false; return val.ptr }(), { let val = authors; val.isOwned = false; return val.ptr }(), { if let val = year_range { val.isOwned = false; return val.ptr } else { return nil } }(), { let rustString = entry_types.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class BibtexMetadataRefMut: BibtexMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class BibtexMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension BibtexMetadataRef {
    public func entry_count() -> UInt {
        __swift_bridge__$BibtexMetadata$entry_count(ptr)
    }

    public func citation_keys() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$BibtexMetadata$citation_keys(ptr))
    }

    public func authors() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$BibtexMetadata$authors(ptr))
    }

    public func year_range() -> Optional<YearRange> {
        { let val = __swift_bridge__$BibtexMetadata$year_range(ptr); if val != nil { return YearRange(ptr: val!) } else { return nil } }()
    }

    public func entry_types() -> RustString {
        RustString(ptr: __swift_bridge__$BibtexMetadata$entry_types(ptr))
    }
}
extension BibtexMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_BibtexMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_BibtexMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: BibtexMetadata) {
        __swift_bridge__$Vec_BibtexMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_BibtexMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (BibtexMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BibtexMetadataRef> {
        let pointer = __swift_bridge__$Vec_BibtexMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BibtexMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BibtexMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_BibtexMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BibtexMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<BibtexMetadataRef> {
        UnsafePointer<BibtexMetadataRef>(OpaquePointer(__swift_bridge__$Vec_BibtexMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_BibtexMetadata$len(vecPtr)
    }
}


public class CitationMetadata: CitationMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CitationMetadata$_free(ptr)
        }
    }
}
extension CitationMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ citation_count: UInt, _ format: Optional<GenericIntoRustString>, _ authors: RustVec<GenericIntoRustString>, _ year_range: Optional<YearRange>, _ dois: RustVec<GenericIntoRustString>, _ keywords: RustVec<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$CitationMetadata$new(citation_count, { if let rustString = optionalStringIntoRustString(format) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let val = authors; val.isOwned = false; return val.ptr }(), { if let val = year_range { val.isOwned = false; return val.ptr } else { return nil } }(), { let val = dois; val.isOwned = false; return val.ptr }(), { let val = keywords; val.isOwned = false; return val.ptr }()))
    }
}
public class CitationMetadataRefMut: CitationMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CitationMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CitationMetadataRef {
    public func citation_count() -> UInt {
        __swift_bridge__$CitationMetadata$citation_count(ptr)
    }

    public func format() -> Optional<RustString> {
        { let val = __swift_bridge__$CitationMetadata$format(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func authors() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$CitationMetadata$authors(ptr))
    }

    public func year_range() -> Optional<YearRange> {
        { let val = __swift_bridge__$CitationMetadata$year_range(ptr); if val != nil { return YearRange(ptr: val!) } else { return nil } }()
    }

    public func dois() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$CitationMetadata$dois(ptr))
    }

    public func keywords() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$CitationMetadata$keywords(ptr))
    }
}
extension CitationMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CitationMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CitationMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CitationMetadata) {
        __swift_bridge__$Vec_CitationMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CitationMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CitationMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CitationMetadataRef> {
        let pointer = __swift_bridge__$Vec_CitationMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CitationMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CitationMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_CitationMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CitationMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CitationMetadataRef> {
        UnsafePointer<CitationMetadataRef>(OpaquePointer(__swift_bridge__$Vec_CitationMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CitationMetadata$len(vecPtr)
    }
}


public class YearRange: YearRangeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$YearRange$_free(ptr)
        }
    }
}
public class YearRangeRefMut: YearRangeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class YearRangeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension YearRangeRef {
    public func min() -> Optional<UInt32> {
        __swift_bridge__$YearRange$min(ptr).intoSwiftRepr()
    }

    public func max() -> Optional<UInt32> {
        __swift_bridge__$YearRange$max(ptr).intoSwiftRepr()
    }

    public func years() -> RustVec<UInt32> {
        RustVec(ptr: __swift_bridge__$YearRange$years(ptr))
    }
}
extension YearRange: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_YearRange$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_YearRange$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: YearRange) {
        __swift_bridge__$Vec_YearRange$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_YearRange$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (YearRange(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<YearRangeRef> {
        let pointer = __swift_bridge__$Vec_YearRange$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return YearRangeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<YearRangeRefMut> {
        let pointer = __swift_bridge__$Vec_YearRange$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return YearRangeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<YearRangeRef> {
        UnsafePointer<YearRangeRef>(OpaquePointer(__swift_bridge__$Vec_YearRange$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_YearRange$len(vecPtr)
    }
}


public class FictionBookMetadata: FictionBookMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$FictionBookMetadata$_free(ptr)
        }
    }
}
extension FictionBookMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ genres: RustVec<GenericIntoRustString>, _ sequences: RustVec<GenericIntoRustString>, _ annotation: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$FictionBookMetadata$new({ let val = genres; val.isOwned = false; return val.ptr }(), { let val = sequences; val.isOwned = false; return val.ptr }(), { if let rustString = optionalStringIntoRustString(annotation) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class FictionBookMetadataRefMut: FictionBookMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FictionBookMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FictionBookMetadataRef {
    public func genres() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$FictionBookMetadata$genres(ptr))
    }

    public func sequences() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$FictionBookMetadata$sequences(ptr))
    }

    public func annotation() -> Optional<RustString> {
        { let val = __swift_bridge__$FictionBookMetadata$annotation(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension FictionBookMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_FictionBookMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_FictionBookMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: FictionBookMetadata) {
        __swift_bridge__$Vec_FictionBookMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_FictionBookMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (FictionBookMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FictionBookMetadataRef> {
        let pointer = __swift_bridge__$Vec_FictionBookMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FictionBookMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FictionBookMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_FictionBookMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FictionBookMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FictionBookMetadataRef> {
        UnsafePointer<FictionBookMetadataRef>(OpaquePointer(__swift_bridge__$Vec_FictionBookMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_FictionBookMetadata$len(vecPtr)
    }
}


public class DbfMetadata: DbfMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DbfMetadata$_free(ptr)
        }
    }
}
extension DbfMetadata {
    public convenience init(_ record_count: UInt, _ field_count: UInt, _ fields: RustVec<DbfFieldInfo>) {
        self.init(ptr: __swift_bridge__$DbfMetadata$new(record_count, field_count, { let val = fields; val.isOwned = false; return val.ptr }()))
    }
}
public class DbfMetadataRefMut: DbfMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DbfMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DbfMetadataRef {
    public func record_count() -> UInt {
        __swift_bridge__$DbfMetadata$record_count(ptr)
    }

    public func field_count() -> UInt {
        __swift_bridge__$DbfMetadata$field_count(ptr)
    }

    public func fields() -> RustVec<DbfFieldInfo> {
        RustVec(ptr: __swift_bridge__$DbfMetadata$fields(ptr))
    }
}
extension DbfMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DbfMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DbfMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DbfMetadata) {
        __swift_bridge__$Vec_DbfMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DbfMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DbfMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DbfMetadataRef> {
        let pointer = __swift_bridge__$Vec_DbfMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DbfMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DbfMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_DbfMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DbfMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DbfMetadataRef> {
        UnsafePointer<DbfMetadataRef>(OpaquePointer(__swift_bridge__$Vec_DbfMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DbfMetadata$len(vecPtr)
    }
}


public class DbfFieldInfo: DbfFieldInfoRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DbfFieldInfo$_free(ptr)
        }
    }
}
public class DbfFieldInfoRefMut: DbfFieldInfoRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DbfFieldInfoRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DbfFieldInfoRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$DbfFieldInfo$name(ptr))
    }

    public func field_type() -> RustString {
        RustString(ptr: __swift_bridge__$DbfFieldInfo$field_type(ptr))
    }
}
extension DbfFieldInfo: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DbfFieldInfo$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DbfFieldInfo$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DbfFieldInfo) {
        __swift_bridge__$Vec_DbfFieldInfo$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DbfFieldInfo$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DbfFieldInfo(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DbfFieldInfoRef> {
        let pointer = __swift_bridge__$Vec_DbfFieldInfo$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DbfFieldInfoRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DbfFieldInfoRefMut> {
        let pointer = __swift_bridge__$Vec_DbfFieldInfo$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DbfFieldInfoRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DbfFieldInfoRef> {
        UnsafePointer<DbfFieldInfoRef>(OpaquePointer(__swift_bridge__$Vec_DbfFieldInfo$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DbfFieldInfo$len(vecPtr)
    }
}


public class JatsMetadata: JatsMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$JatsMetadata$_free(ptr)
        }
    }
}
extension JatsMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ copyright: Optional<GenericIntoRustString>, _ license: Optional<GenericIntoRustString>, _ history_dates: GenericIntoRustString, _ contributor_roles: RustVec<ContributorRole>) {
        self.init(ptr: __swift_bridge__$JatsMetadata$new({ if let rustString = optionalStringIntoRustString(copyright) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(license) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let rustString = history_dates.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = contributor_roles; val.isOwned = false; return val.ptr }()))
    }
}
public class JatsMetadataRefMut: JatsMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class JatsMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension JatsMetadataRef {
    public func copyright() -> Optional<RustString> {
        { let val = __swift_bridge__$JatsMetadata$copyright(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func license() -> Optional<RustString> {
        { let val = __swift_bridge__$JatsMetadata$license(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func history_dates() -> RustString {
        RustString(ptr: __swift_bridge__$JatsMetadata$history_dates(ptr))
    }

    public func contributor_roles() -> RustVec<ContributorRole> {
        RustVec(ptr: __swift_bridge__$JatsMetadata$contributor_roles(ptr))
    }
}
extension JatsMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_JatsMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_JatsMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: JatsMetadata) {
        __swift_bridge__$Vec_JatsMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_JatsMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (JatsMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<JatsMetadataRef> {
        let pointer = __swift_bridge__$Vec_JatsMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return JatsMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<JatsMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_JatsMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return JatsMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<JatsMetadataRef> {
        UnsafePointer<JatsMetadataRef>(OpaquePointer(__swift_bridge__$Vec_JatsMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_JatsMetadata$len(vecPtr)
    }
}


public class ContributorRole: ContributorRoleRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ContributorRole$_free(ptr)
        }
    }
}
public class ContributorRoleRefMut: ContributorRoleRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ContributorRoleRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ContributorRoleRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$ContributorRole$name(ptr))
    }

    public func role() -> Optional<RustString> {
        { let val = __swift_bridge__$ContributorRole$role(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension ContributorRole: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ContributorRole$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ContributorRole$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ContributorRole) {
        __swift_bridge__$Vec_ContributorRole$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ContributorRole$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ContributorRole(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContributorRoleRef> {
        let pointer = __swift_bridge__$Vec_ContributorRole$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContributorRoleRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContributorRoleRefMut> {
        let pointer = __swift_bridge__$Vec_ContributorRole$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContributorRoleRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ContributorRoleRef> {
        UnsafePointer<ContributorRoleRef>(OpaquePointer(__swift_bridge__$Vec_ContributorRole$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ContributorRole$len(vecPtr)
    }
}


public class EpubMetadata: EpubMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EpubMetadata$_free(ptr)
        }
    }
}
extension EpubMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ coverage: Optional<GenericIntoRustString>, _ dc_format: Optional<GenericIntoRustString>, _ relation: Optional<GenericIntoRustString>, _ source: Optional<GenericIntoRustString>, _ dc_type: Optional<GenericIntoRustString>, _ cover_image: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$EpubMetadata$new({ if let rustString = optionalStringIntoRustString(coverage) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(dc_format) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(relation) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(source) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(dc_type) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(cover_image) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class EpubMetadataRefMut: EpubMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EpubMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EpubMetadataRef {
    public func coverage() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$coverage(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func dc_format() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$dc_format(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func relation() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$relation(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func source() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$source(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func dc_type() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$dc_type(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func cover_image() -> Optional<RustString> {
        { let val = __swift_bridge__$EpubMetadata$cover_image(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension EpubMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EpubMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EpubMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EpubMetadata) {
        __swift_bridge__$Vec_EpubMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EpubMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EpubMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EpubMetadataRef> {
        let pointer = __swift_bridge__$Vec_EpubMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EpubMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EpubMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_EpubMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EpubMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EpubMetadataRef> {
        UnsafePointer<EpubMetadataRef>(OpaquePointer(__swift_bridge__$Vec_EpubMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EpubMetadata$len(vecPtr)
    }
}


public class PstMetadata: PstMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PstMetadata$_free(ptr)
        }
    }
}
extension PstMetadata {
    public convenience init(_ message_count: UInt) {
        self.init(ptr: __swift_bridge__$PstMetadata$new(message_count))
    }
}
public class PstMetadataRefMut: PstMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PstMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PstMetadataRef {
    public func message_count() -> UInt {
        __swift_bridge__$PstMetadata$message_count(ptr)
    }
}
extension PstMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PstMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PstMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PstMetadata) {
        __swift_bridge__$Vec_PstMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PstMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PstMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PstMetadataRef> {
        let pointer = __swift_bridge__$Vec_PstMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PstMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PstMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_PstMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PstMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PstMetadataRef> {
        UnsafePointer<PstMetadataRef>(OpaquePointer(__swift_bridge__$Vec_PstMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PstMetadata$len(vecPtr)
    }
}


public class OcrConfidence: OcrConfidenceRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrConfidence$_free(ptr)
        }
    }
}
extension OcrConfidence {
    public convenience init(_ detection: Optional<Double>, _ recognition: Double) {
        self.init(ptr: __swift_bridge__$OcrConfidence$new(detection.intoFfiRepr(), recognition))
    }
}
public class OcrConfidenceRefMut: OcrConfidenceRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrConfidenceRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrConfidenceRef {
    public func detection() -> Optional<Double> {
        __swift_bridge__$OcrConfidence$detection(ptr).intoSwiftRepr()
    }

    public func recognition() -> Double {
        __swift_bridge__$OcrConfidence$recognition(ptr)
    }
}
extension OcrConfidence: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrConfidence$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrConfidence$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrConfidence) {
        __swift_bridge__$Vec_OcrConfidence$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrConfidence$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrConfidence(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrConfidenceRef> {
        let pointer = __swift_bridge__$Vec_OcrConfidence$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrConfidenceRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrConfidenceRefMut> {
        let pointer = __swift_bridge__$Vec_OcrConfidence$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrConfidenceRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrConfidenceRef> {
        UnsafePointer<OcrConfidenceRef>(OpaquePointer(__swift_bridge__$Vec_OcrConfidence$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrConfidence$len(vecPtr)
    }
}


public class OcrRotation: OcrRotationRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrRotation$_free(ptr)
        }
    }
}
public class OcrRotationRefMut: OcrRotationRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrRotationRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrRotationRef {
    public func angle_degrees() -> Double {
        __swift_bridge__$OcrRotation$angle_degrees(ptr)
    }

    public func confidence() -> Optional<Double> {
        __swift_bridge__$OcrRotation$confidence(ptr).intoSwiftRepr()
    }
}
extension OcrRotation: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrRotation$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrRotation$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrRotation) {
        __swift_bridge__$Vec_OcrRotation$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrRotation$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrRotation(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrRotationRef> {
        let pointer = __swift_bridge__$Vec_OcrRotation$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrRotationRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrRotationRefMut> {
        let pointer = __swift_bridge__$Vec_OcrRotation$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrRotationRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrRotationRef> {
        UnsafePointer<OcrRotationRef>(OpaquePointer(__swift_bridge__$Vec_OcrRotation$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrRotation$len(vecPtr)
    }
}


public class OcrElement: OcrElementRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrElement$_free(ptr)
        }
    }
}
extension OcrElement {
    public convenience init<GenericIntoRustString: IntoRustString>(_ text: GenericIntoRustString, _ geometry: OcrBoundingGeometry, _ confidence: OcrConfidence, _ level: OcrElementLevel, _ rotation: Optional<OcrRotation>, _ page_number: UInt, _ parent_id: Optional<GenericIntoRustString>, _ backend_metadata: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$OcrElement$new({ let rustString = text.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), {geometry.isOwned = false; return geometry.ptr;}(), {confidence.isOwned = false; return confidence.ptr;}(), {level.isOwned = false; return level.ptr;}(), { if let val = rotation { val.isOwned = false; return val.ptr } else { return nil } }(), page_number, { if let rustString = optionalStringIntoRustString(parent_id) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { let rustString = backend_metadata.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class OcrElementRefMut: OcrElementRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrElementRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrElementRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$OcrElement$text(ptr))
    }

    public func geometry() -> OcrBoundingGeometry {
        OcrBoundingGeometry(ptr: __swift_bridge__$OcrElement$geometry(ptr))
    }

    public func confidence() -> OcrConfidence {
        OcrConfidence(ptr: __swift_bridge__$OcrElement$confidence(ptr))
    }

    public func level() -> OcrElementLevel {
        OcrElementLevel(ptr: __swift_bridge__$OcrElement$level(ptr))
    }

    public func rotation() -> Optional<OcrRotation> {
        { let val = __swift_bridge__$OcrElement$rotation(ptr); if val != nil { return OcrRotation(ptr: val!) } else { return nil } }()
    }

    public func page_number() -> UInt {
        __swift_bridge__$OcrElement$page_number(ptr)
    }

    public func parent_id() -> Optional<RustString> {
        { let val = __swift_bridge__$OcrElement$parent_id(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func backend_metadata() -> RustString {
        RustString(ptr: __swift_bridge__$OcrElement$backend_metadata(ptr))
    }
}
extension OcrElement: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrElement$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrElement$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrElement) {
        __swift_bridge__$Vec_OcrElement$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrElement$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrElement(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementRef> {
        let pointer = __swift_bridge__$Vec_OcrElement$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementRefMut> {
        let pointer = __swift_bridge__$Vec_OcrElement$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrElementRef> {
        UnsafePointer<OcrElementRef>(OpaquePointer(__swift_bridge__$Vec_OcrElement$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrElement$len(vecPtr)
    }
}


public class OcrElementConfig: OcrElementConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrElementConfig$_free(ptr)
        }
    }
}
extension OcrElementConfig {
    public convenience init(_ include_elements: Bool, _ min_level: OcrElementLevel, _ min_confidence: Double, _ build_hierarchy: Bool) {
        self.init(ptr: __swift_bridge__$OcrElementConfig$new(include_elements, {min_level.isOwned = false; return min_level.ptr;}(), min_confidence, build_hierarchy))
    }
}
public class OcrElementConfigRefMut: OcrElementConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrElementConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrElementConfigRef {
    public func include_elements() -> Bool {
        __swift_bridge__$OcrElementConfig$include_elements(ptr)
    }

    public func min_level() -> OcrElementLevel {
        OcrElementLevel(ptr: __swift_bridge__$OcrElementConfig$min_level(ptr))
    }

    public func min_confidence() -> Double {
        __swift_bridge__$OcrElementConfig$min_confidence(ptr)
    }

    public func build_hierarchy() -> Bool {
        __swift_bridge__$OcrElementConfig$build_hierarchy(ptr)
    }
}
extension OcrElementConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrElementConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrElementConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrElementConfig) {
        __swift_bridge__$Vec_OcrElementConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrElementConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrElementConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementConfigRef> {
        let pointer = __swift_bridge__$Vec_OcrElementConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementConfigRefMut> {
        let pointer = __swift_bridge__$Vec_OcrElementConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrElementConfigRef> {
        UnsafePointer<OcrElementConfigRef>(OpaquePointer(__swift_bridge__$Vec_OcrElementConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrElementConfig$len(vecPtr)
    }
}


public class PageStructure: PageStructureRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageStructure$_free(ptr)
        }
    }
}
public class PageStructureRefMut: PageStructureRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageStructureRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageStructureRef {
    public func total_count() -> UInt {
        __swift_bridge__$PageStructure$total_count(ptr)
    }

    public func unit_type() -> PageUnitType {
        PageUnitType(ptr: __swift_bridge__$PageStructure$unit_type(ptr))
    }

    public func boundaries() -> Optional<RustVec<PageBoundary>> {
        { let val = __swift_bridge__$PageStructure$boundaries(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func pages() -> Optional<RustVec<PageInfo>> {
        { let val = __swift_bridge__$PageStructure$pages(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension PageStructure: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageStructure$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageStructure$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageStructure) {
        __swift_bridge__$Vec_PageStructure$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageStructure$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageStructure(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageStructureRef> {
        let pointer = __swift_bridge__$Vec_PageStructure$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageStructureRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageStructureRefMut> {
        let pointer = __swift_bridge__$Vec_PageStructure$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageStructureRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageStructureRef> {
        UnsafePointer<PageStructureRef>(OpaquePointer(__swift_bridge__$Vec_PageStructure$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageStructure$len(vecPtr)
    }
}


public class PageBoundary: PageBoundaryRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageBoundary$_free(ptr)
        }
    }
}
public class PageBoundaryRefMut: PageBoundaryRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageBoundaryRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageBoundaryRef {
    public func byte_start() -> UInt {
        __swift_bridge__$PageBoundary$byte_start(ptr)
    }

    public func byte_end() -> UInt {
        __swift_bridge__$PageBoundary$byte_end(ptr)
    }

    public func page_number() -> UInt {
        __swift_bridge__$PageBoundary$page_number(ptr)
    }
}
extension PageBoundary: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageBoundary$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageBoundary$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageBoundary) {
        __swift_bridge__$Vec_PageBoundary$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageBoundary$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageBoundary(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageBoundaryRef> {
        let pointer = __swift_bridge__$Vec_PageBoundary$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageBoundaryRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageBoundaryRefMut> {
        let pointer = __swift_bridge__$Vec_PageBoundary$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageBoundaryRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageBoundaryRef> {
        UnsafePointer<PageBoundaryRef>(OpaquePointer(__swift_bridge__$Vec_PageBoundary$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageBoundary$len(vecPtr)
    }
}


public class PageInfo: PageInfoRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageInfo$_free(ptr)
        }
    }
}
public class PageInfoRefMut: PageInfoRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageInfoRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageInfoRef {
    public func number() -> UInt {
        __swift_bridge__$PageInfo$number(ptr)
    }

    public func title() -> Optional<RustString> {
        { let val = __swift_bridge__$PageInfo$title(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func dimensions() -> Optional<RustVec<Double>> {
        { let val = __swift_bridge__$PageInfo$dimensions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func image_count() -> Optional<UInt> {
        __swift_bridge__$PageInfo$image_count(ptr).intoSwiftRepr()
    }

    public func table_count() -> Optional<UInt> {
        __swift_bridge__$PageInfo$table_count(ptr).intoSwiftRepr()
    }

    public func hidden() -> Optional<Bool> {
        __swift_bridge__$PageInfo$hidden(ptr).intoSwiftRepr()
    }

    public func is_blank() -> Optional<Bool> {
        __swift_bridge__$PageInfo$is_blank(ptr).intoSwiftRepr()
    }

    public func has_vector_graphics() -> Bool {
        __swift_bridge__$PageInfo$has_vector_graphics(ptr)
    }
}
extension PageInfo: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageInfo$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageInfo$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageInfo) {
        __swift_bridge__$Vec_PageInfo$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageInfo$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageInfo(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageInfoRef> {
        let pointer = __swift_bridge__$Vec_PageInfo$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageInfoRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageInfoRefMut> {
        let pointer = __swift_bridge__$Vec_PageInfo$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageInfoRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageInfoRef> {
        UnsafePointer<PageInfoRef>(OpaquePointer(__swift_bridge__$Vec_PageInfo$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageInfo$len(vecPtr)
    }
}


public class PageContent: PageContentRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageContent$_free(ptr)
        }
    }
}
public class PageContentRefMut: PageContentRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageContentRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageContentRef {
    public func page_number() -> UInt {
        __swift_bridge__$PageContent$page_number(ptr)
    }

    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$PageContent$content(ptr))
    }

    public func tables() -> RustVec<Table> {
        RustVec(ptr: __swift_bridge__$PageContent$tables(ptr))
    }

    public func images() -> RustVec<ExtractedImage> {
        RustVec(ptr: __swift_bridge__$PageContent$images(ptr))
    }

    public func hierarchy() -> Optional<PageHierarchy> {
        { let val = __swift_bridge__$PageContent$hierarchy(ptr); if val != nil { return PageHierarchy(ptr: val!) } else { return nil } }()
    }

    public func is_blank() -> Optional<Bool> {
        __swift_bridge__$PageContent$is_blank(ptr).intoSwiftRepr()
    }

    public func layout_regions() -> Optional<RustVec<LayoutRegion>> {
        { let val = __swift_bridge__$PageContent$layout_regions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension PageContent: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageContent$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageContent$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageContent) {
        __swift_bridge__$Vec_PageContent$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageContent$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageContent(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageContentRef> {
        let pointer = __swift_bridge__$Vec_PageContent$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageContentRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageContentRefMut> {
        let pointer = __swift_bridge__$Vec_PageContent$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageContentRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageContentRef> {
        UnsafePointer<PageContentRef>(OpaquePointer(__swift_bridge__$Vec_PageContent$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageContent$len(vecPtr)
    }
}


public class LayoutRegion: LayoutRegionRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LayoutRegion$_free(ptr)
        }
    }
}
extension LayoutRegion {
    public convenience init<GenericIntoRustString: IntoRustString>(_ class_name: GenericIntoRustString, _ confidence: Double, _ bounding_box: GenericIntoRustString, _ area_fraction: Double) {
        self.init(ptr: __swift_bridge__$LayoutRegion$new({ let rustString = class_name.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), confidence, { let rustString = bounding_box.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), area_fraction))
    }
}
public class LayoutRegionRefMut: LayoutRegionRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LayoutRegionRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LayoutRegionRef {
    public func class_name() -> RustString {
        RustString(ptr: __swift_bridge__$LayoutRegion$class_name(ptr))
    }

    public func confidence() -> Double {
        __swift_bridge__$LayoutRegion$confidence(ptr)
    }

    public func bounding_box() -> RustString {
        RustString(ptr: __swift_bridge__$LayoutRegion$bounding_box(ptr))
    }

    public func area_fraction() -> Double {
        __swift_bridge__$LayoutRegion$area_fraction(ptr)
    }
}
extension LayoutRegion: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LayoutRegion$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LayoutRegion$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LayoutRegion) {
        __swift_bridge__$Vec_LayoutRegion$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LayoutRegion$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LayoutRegion(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutRegionRef> {
        let pointer = __swift_bridge__$Vec_LayoutRegion$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutRegionRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutRegionRefMut> {
        let pointer = __swift_bridge__$Vec_LayoutRegion$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutRegionRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LayoutRegionRef> {
        UnsafePointer<LayoutRegionRef>(OpaquePointer(__swift_bridge__$Vec_LayoutRegion$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LayoutRegion$len(vecPtr)
    }
}


public class PageHierarchy: PageHierarchyRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageHierarchy$_free(ptr)
        }
    }
}
public class PageHierarchyRefMut: PageHierarchyRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageHierarchyRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageHierarchyRef {
    public func block_count() -> UInt {
        __swift_bridge__$PageHierarchy$block_count(ptr)
    }

    public func blocks() -> RustVec<HierarchicalBlock> {
        RustVec(ptr: __swift_bridge__$PageHierarchy$blocks(ptr))
    }
}
extension PageHierarchy: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageHierarchy$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageHierarchy$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageHierarchy) {
        __swift_bridge__$Vec_PageHierarchy$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageHierarchy$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageHierarchy(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageHierarchyRef> {
        let pointer = __swift_bridge__$Vec_PageHierarchy$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageHierarchyRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageHierarchyRefMut> {
        let pointer = __swift_bridge__$Vec_PageHierarchy$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageHierarchyRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageHierarchyRef> {
        UnsafePointer<PageHierarchyRef>(OpaquePointer(__swift_bridge__$Vec_PageHierarchy$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageHierarchy$len(vecPtr)
    }
}


public class HierarchicalBlock: HierarchicalBlockRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HierarchicalBlock$_free(ptr)
        }
    }
}
public class HierarchicalBlockRefMut: HierarchicalBlockRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HierarchicalBlockRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HierarchicalBlockRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$HierarchicalBlock$text(ptr))
    }

    public func font_size() -> Float {
        __swift_bridge__$HierarchicalBlock$font_size(ptr)
    }

    public func level() -> RustString {
        RustString(ptr: __swift_bridge__$HierarchicalBlock$level(ptr))
    }

    public func bbox() -> Optional<RustVec<Float>> {
        { let val = __swift_bridge__$HierarchicalBlock$bbox(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension HierarchicalBlock: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HierarchicalBlock$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HierarchicalBlock$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HierarchicalBlock) {
        __swift_bridge__$Vec_HierarchicalBlock$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HierarchicalBlock$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HierarchicalBlock(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HierarchicalBlockRef> {
        let pointer = __swift_bridge__$Vec_HierarchicalBlock$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HierarchicalBlockRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HierarchicalBlockRefMut> {
        let pointer = __swift_bridge__$Vec_HierarchicalBlock$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HierarchicalBlockRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HierarchicalBlockRef> {
        UnsafePointer<HierarchicalBlockRef>(OpaquePointer(__swift_bridge__$Vec_HierarchicalBlock$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HierarchicalBlock$len(vecPtr)
    }
}


public class Table: TableRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Table$_free(ptr)
        }
    }
}
extension Table {
    public convenience init<GenericIntoRustString: IntoRustString>(_ cells: GenericIntoRustString, _ markdown: GenericIntoRustString, _ page_number: UInt, _ bounding_box: Optional<GenericIntoRustString>) {
        self.init(ptr: __swift_bridge__$Table$new({ let rustString = cells.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = markdown.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), page_number, { if let rustString = optionalStringIntoRustString(bounding_box) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()))
    }
}
public class TableRefMut: TableRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TableRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TableRef {
    public func cells() -> RustString {
        RustString(ptr: __swift_bridge__$Table$cells(ptr))
    }

    public func markdown() -> RustString {
        RustString(ptr: __swift_bridge__$Table$markdown(ptr))
    }

    public func page_number() -> UInt {
        __swift_bridge__$Table$page_number(ptr)
    }

    public func bounding_box() -> Optional<RustString> {
        { let val = __swift_bridge__$Table$bounding_box(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension Table: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Table$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Table$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Table) {
        __swift_bridge__$Vec_Table$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Table$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Table(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableRef> {
        let pointer = __swift_bridge__$Vec_Table$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableRefMut> {
        let pointer = __swift_bridge__$Vec_Table$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TableRef> {
        UnsafePointer<TableRef>(OpaquePointer(__swift_bridge__$Vec_Table$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Table$len(vecPtr)
    }
}


public class TableCell: TableCellRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TableCell$_free(ptr)
        }
    }
}
extension TableCell {
    public convenience init<GenericIntoRustString: IntoRustString>(_ content: GenericIntoRustString, _ row_span: UInt, _ col_span: UInt, _ is_header: Bool) {
        self.init(ptr: __swift_bridge__$TableCell$new({ let rustString = content.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), row_span, col_span, is_header))
    }
}
public class TableCellRefMut: TableCellRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TableCellRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TableCellRef {
    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$TableCell$content(ptr))
    }

    public func row_span() -> UInt {
        __swift_bridge__$TableCell$row_span(ptr)
    }

    public func col_span() -> UInt {
        __swift_bridge__$TableCell$col_span(ptr)
    }

    public func is_header() -> Bool {
        __swift_bridge__$TableCell$is_header(ptr)
    }
}
extension TableCell: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TableCell$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TableCell$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TableCell) {
        __swift_bridge__$Vec_TableCell$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TableCell$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TableCell(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableCellRef> {
        let pointer = __swift_bridge__$Vec_TableCell$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableCellRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableCellRefMut> {
        let pointer = __swift_bridge__$Vec_TableCell$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableCellRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TableCellRef> {
        UnsafePointer<TableCellRef>(OpaquePointer(__swift_bridge__$Vec_TableCell$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TableCell$len(vecPtr)
    }
}


public class Uri: UriRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Uri$_free(ptr)
        }
    }
}
public class UriRefMut: UriRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class UriRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension UriRef {
    public func url() -> RustString {
        RustString(ptr: __swift_bridge__$Uri$url(ptr))
    }

    public func label() -> Optional<RustString> {
        { let val = __swift_bridge__$Uri$label(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func page() -> Optional<UInt32> {
        __swift_bridge__$Uri$page(ptr).intoSwiftRepr()
    }

    public func kind() -> UriKind {
        UriKind(ptr: __swift_bridge__$Uri$kind(ptr))
    }
}
extension Uri: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Uri$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Uri$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Uri) {
        __swift_bridge__$Vec_Uri$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Uri$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Uri(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UriRef> {
        let pointer = __swift_bridge__$Vec_Uri$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UriRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UriRefMut> {
        let pointer = __swift_bridge__$Vec_Uri$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UriRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<UriRef> {
        UnsafePointer<UriRef>(OpaquePointer(__swift_bridge__$Vec_Uri$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Uri$len(vecPtr)
    }
}


public class StringBufferPool: StringBufferPoolRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StringBufferPool$_free(ptr)
        }
    }
}
public class StringBufferPoolRefMut: StringBufferPoolRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StringBufferPoolRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StringBufferPool: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StringBufferPool$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StringBufferPool$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StringBufferPool) {
        __swift_bridge__$Vec_StringBufferPool$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StringBufferPool$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StringBufferPool(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StringBufferPoolRef> {
        let pointer = __swift_bridge__$Vec_StringBufferPool$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StringBufferPoolRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StringBufferPoolRefMut> {
        let pointer = __swift_bridge__$Vec_StringBufferPool$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StringBufferPoolRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StringBufferPoolRef> {
        UnsafePointer<StringBufferPoolRef>(OpaquePointer(__swift_bridge__$Vec_StringBufferPool$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StringBufferPool$len(vecPtr)
    }
}


public class ByteBufferPool: ByteBufferPoolRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ByteBufferPool$_free(ptr)
        }
    }
}
public class ByteBufferPoolRefMut: ByteBufferPoolRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ByteBufferPoolRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ByteBufferPool: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ByteBufferPool$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ByteBufferPool$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ByteBufferPool) {
        __swift_bridge__$Vec_ByteBufferPool$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ByteBufferPool$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ByteBufferPool(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ByteBufferPoolRef> {
        let pointer = __swift_bridge__$Vec_ByteBufferPool$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ByteBufferPoolRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ByteBufferPoolRefMut> {
        let pointer = __swift_bridge__$Vec_ByteBufferPool$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ByteBufferPoolRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ByteBufferPoolRef> {
        UnsafePointer<ByteBufferPoolRef>(OpaquePointer(__swift_bridge__$Vec_ByteBufferPool$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ByteBufferPool$len(vecPtr)
    }
}


public class TracingLayer: TracingLayerRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TracingLayer$_free(ptr)
        }
    }
}
public class TracingLayerRefMut: TracingLayerRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TracingLayerRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TracingLayer: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TracingLayer$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TracingLayer$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TracingLayer) {
        __swift_bridge__$Vec_TracingLayer$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TracingLayer$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TracingLayer(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TracingLayerRef> {
        let pointer = __swift_bridge__$Vec_TracingLayer$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TracingLayerRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TracingLayerRefMut> {
        let pointer = __swift_bridge__$Vec_TracingLayer$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TracingLayerRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TracingLayerRef> {
        UnsafePointer<TracingLayerRef>(OpaquePointer(__swift_bridge__$Vec_TracingLayer$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TracingLayer$len(vecPtr)
    }
}


public class ApiDoc: ApiDocRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ApiDoc$_free(ptr)
        }
    }
}
public class ApiDocRefMut: ApiDocRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ApiDocRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ApiDoc: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ApiDoc$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ApiDoc$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ApiDoc) {
        __swift_bridge__$Vec_ApiDoc$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ApiDoc$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ApiDoc(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ApiDocRef> {
        let pointer = __swift_bridge__$Vec_ApiDoc$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ApiDocRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ApiDocRefMut> {
        let pointer = __swift_bridge__$Vec_ApiDoc$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ApiDocRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ApiDocRef> {
        UnsafePointer<ApiDocRef>(OpaquePointer(__swift_bridge__$Vec_ApiDoc$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ApiDoc$len(vecPtr)
    }
}


public class InfoResponse: InfoResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$InfoResponse$_free(ptr)
        }
    }
}
public class InfoResponseRefMut: InfoResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class InfoResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension InfoResponseRef {
    public func version() -> RustString {
        RustString(ptr: __swift_bridge__$InfoResponse$version(ptr))
    }

    public func rust_backend() -> Bool {
        __swift_bridge__$InfoResponse$rust_backend(ptr)
    }
}
extension InfoResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_InfoResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_InfoResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: InfoResponse) {
        __swift_bridge__$Vec_InfoResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_InfoResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (InfoResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InfoResponseRef> {
        let pointer = __swift_bridge__$Vec_InfoResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InfoResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InfoResponseRefMut> {
        let pointer = __swift_bridge__$Vec_InfoResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InfoResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<InfoResponseRef> {
        UnsafePointer<InfoResponseRef>(OpaquePointer(__swift_bridge__$Vec_InfoResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_InfoResponse$len(vecPtr)
    }
}


public class ExtractResponse: ExtractResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractResponse$_free(ptr)
        }
    }
}
public class ExtractResponseRefMut: ExtractResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractResponse) {
        __swift_bridge__$Vec_ExtractResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractResponseRef> {
        let pointer = __swift_bridge__$Vec_ExtractResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractResponseRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractResponseRef> {
        UnsafePointer<ExtractResponseRef>(OpaquePointer(__swift_bridge__$Vec_ExtractResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractResponse$len(vecPtr)
    }
}


public class EmbedRequest: EmbedRequestRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbedRequest$_free(ptr)
        }
    }
}
public class EmbedRequestRefMut: EmbedRequestRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbedRequestRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbedRequestRef {
    public func texts() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmbedRequest$texts(ptr))
    }

    public func config() -> Optional<EmbeddingConfig> {
        { let val = __swift_bridge__$EmbedRequest$config(ptr); if val != nil { return EmbeddingConfig(ptr: val!) } else { return nil } }()
    }
}
extension EmbedRequest: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbedRequest$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbedRequest$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbedRequest) {
        __swift_bridge__$Vec_EmbedRequest$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbedRequest$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbedRequest(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedRequestRef> {
        let pointer = __swift_bridge__$Vec_EmbedRequest$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedRequestRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedRequestRefMut> {
        let pointer = __swift_bridge__$Vec_EmbedRequest$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedRequestRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbedRequestRef> {
        UnsafePointer<EmbedRequestRef>(OpaquePointer(__swift_bridge__$Vec_EmbedRequest$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbedRequest$len(vecPtr)
    }
}


public class EmbedResponse: EmbedResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbedResponse$_free(ptr)
        }
    }
}
public class EmbedResponseRefMut: EmbedResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbedResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbedResponseRef {
    public func embeddings() -> RustString {
        RustString(ptr: __swift_bridge__$EmbedResponse$embeddings(ptr))
    }

    public func model() -> RustString {
        RustString(ptr: __swift_bridge__$EmbedResponse$model(ptr))
    }

    public func dimensions() -> UInt {
        __swift_bridge__$EmbedResponse$dimensions(ptr)
    }

    public func count() -> UInt {
        __swift_bridge__$EmbedResponse$count(ptr)
    }
}
extension EmbedResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbedResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbedResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbedResponse) {
        __swift_bridge__$Vec_EmbedResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbedResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbedResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedResponseRef> {
        let pointer = __swift_bridge__$Vec_EmbedResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedResponseRefMut> {
        let pointer = __swift_bridge__$Vec_EmbedResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbedResponseRef> {
        UnsafePointer<EmbedResponseRef>(OpaquePointer(__swift_bridge__$Vec_EmbedResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbedResponse$len(vecPtr)
    }
}


public class ChunkRequest: ChunkRequestRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkRequest$_free(ptr)
        }
    }
}
public class ChunkRequestRefMut: ChunkRequestRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkRequestRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkRequestRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$ChunkRequest$text(ptr))
    }

    public func config() -> Optional<RustString> {
        { let val = __swift_bridge__$ChunkRequest$config(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func chunker_type() -> RustString {
        RustString(ptr: __swift_bridge__$ChunkRequest$chunker_type(ptr))
    }
}
extension ChunkRequest: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkRequest$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkRequest$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkRequest) {
        __swift_bridge__$Vec_ChunkRequest$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkRequest$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkRequest(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkRequestRef> {
        let pointer = __swift_bridge__$Vec_ChunkRequest$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkRequestRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkRequestRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkRequest$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkRequestRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkRequestRef> {
        UnsafePointer<ChunkRequestRef>(OpaquePointer(__swift_bridge__$Vec_ChunkRequest$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkRequest$len(vecPtr)
    }
}


public class ChunkResponse: ChunkResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkResponse$_free(ptr)
        }
    }
}
public class ChunkResponseRefMut: ChunkResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkResponseRef {
    public func chunks() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$ChunkResponse$chunks(ptr))
    }

    public func chunk_count() -> UInt {
        __swift_bridge__$ChunkResponse$chunk_count(ptr)
    }

    public func config() -> RustString {
        RustString(ptr: __swift_bridge__$ChunkResponse$config(ptr))
    }

    public func input_size_bytes() -> UInt {
        __swift_bridge__$ChunkResponse$input_size_bytes(ptr)
    }

    public func chunker_type() -> RustString {
        RustString(ptr: __swift_bridge__$ChunkResponse$chunker_type(ptr))
    }
}
extension ChunkResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkResponse) {
        __swift_bridge__$Vec_ChunkResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkResponseRef> {
        let pointer = __swift_bridge__$Vec_ChunkResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkResponseRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkResponseRef> {
        UnsafePointer<ChunkResponseRef>(OpaquePointer(__swift_bridge__$Vec_ChunkResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkResponse$len(vecPtr)
    }
}


public class DetectResponse: DetectResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DetectResponse$_free(ptr)
        }
    }
}
public class DetectResponseRefMut: DetectResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DetectResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DetectResponseRef {
    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$DetectResponse$mime_type(ptr))
    }

    public func filename() -> Optional<RustString> {
        { let val = __swift_bridge__$DetectResponse$filename(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension DetectResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DetectResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DetectResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DetectResponse) {
        __swift_bridge__$Vec_DetectResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DetectResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DetectResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectResponseRef> {
        let pointer = __swift_bridge__$Vec_DetectResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectResponseRefMut> {
        let pointer = __swift_bridge__$Vec_DetectResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DetectResponseRef> {
        UnsafePointer<DetectResponseRef>(OpaquePointer(__swift_bridge__$Vec_DetectResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DetectResponse$len(vecPtr)
    }
}


public class ManifestEntryResponse: ManifestEntryResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ManifestEntryResponse$_free(ptr)
        }
    }
}
public class ManifestEntryResponseRefMut: ManifestEntryResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ManifestEntryResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ManifestEntryResponseRef {
    public func relative_path() -> RustString {
        RustString(ptr: __swift_bridge__$ManifestEntryResponse$relative_path(ptr))
    }

    public func sha256() -> RustString {
        RustString(ptr: __swift_bridge__$ManifestEntryResponse$sha256(ptr))
    }

    public func size_bytes() -> UInt64 {
        __swift_bridge__$ManifestEntryResponse$size_bytes(ptr)
    }

    public func source_url() -> RustString {
        RustString(ptr: __swift_bridge__$ManifestEntryResponse$source_url(ptr))
    }
}
extension ManifestEntryResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ManifestEntryResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ManifestEntryResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ManifestEntryResponse) {
        __swift_bridge__$Vec_ManifestEntryResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ManifestEntryResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ManifestEntryResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ManifestEntryResponseRef> {
        let pointer = __swift_bridge__$Vec_ManifestEntryResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ManifestEntryResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ManifestEntryResponseRefMut> {
        let pointer = __swift_bridge__$Vec_ManifestEntryResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ManifestEntryResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ManifestEntryResponseRef> {
        UnsafePointer<ManifestEntryResponseRef>(OpaquePointer(__swift_bridge__$Vec_ManifestEntryResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ManifestEntryResponse$len(vecPtr)
    }
}


public class ManifestResponse: ManifestResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ManifestResponse$_free(ptr)
        }
    }
}
public class ManifestResponseRefMut: ManifestResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ManifestResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ManifestResponseRef {
    public func kreuzberg_version() -> RustString {
        RustString(ptr: __swift_bridge__$ManifestResponse$kreuzberg_version(ptr))
    }

    public func total_size_bytes() -> UInt64 {
        __swift_bridge__$ManifestResponse$total_size_bytes(ptr)
    }

    public func model_count() -> UInt {
        __swift_bridge__$ManifestResponse$model_count(ptr)
    }

    public func models() -> RustVec<ManifestEntryResponse> {
        RustVec(ptr: __swift_bridge__$ManifestResponse$models(ptr))
    }
}
extension ManifestResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ManifestResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ManifestResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ManifestResponse) {
        __swift_bridge__$Vec_ManifestResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ManifestResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ManifestResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ManifestResponseRef> {
        let pointer = __swift_bridge__$Vec_ManifestResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ManifestResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ManifestResponseRefMut> {
        let pointer = __swift_bridge__$Vec_ManifestResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ManifestResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ManifestResponseRef> {
        UnsafePointer<ManifestResponseRef>(OpaquePointer(__swift_bridge__$Vec_ManifestResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ManifestResponse$len(vecPtr)
    }
}


public class WarmResponse: WarmResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$WarmResponse$_free(ptr)
        }
    }
}
public class WarmResponseRefMut: WarmResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class WarmResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension WarmResponseRef {
    public func cache_dir() -> RustString {
        RustString(ptr: __swift_bridge__$WarmResponse$cache_dir(ptr))
    }

    public func downloaded() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$WarmResponse$downloaded(ptr))
    }

    public func already_cached() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$WarmResponse$already_cached(ptr))
    }
}
extension WarmResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_WarmResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_WarmResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: WarmResponse) {
        __swift_bridge__$Vec_WarmResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_WarmResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (WarmResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<WarmResponseRef> {
        let pointer = __swift_bridge__$Vec_WarmResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return WarmResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<WarmResponseRefMut> {
        let pointer = __swift_bridge__$Vec_WarmResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return WarmResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<WarmResponseRef> {
        UnsafePointer<WarmResponseRef>(OpaquePointer(__swift_bridge__$Vec_WarmResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_WarmResponse$len(vecPtr)
    }
}


public class StructuredExtractionResponse: StructuredExtractionResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StructuredExtractionResponse$_free(ptr)
        }
    }
}
public class StructuredExtractionResponseRefMut: StructuredExtractionResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StructuredExtractionResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StructuredExtractionResponseRef {
    public func structured_output() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredExtractionResponse$structured_output(ptr))
    }

    public func content() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredExtractionResponse$content(ptr))
    }

    public func mime_type() -> RustString {
        RustString(ptr: __swift_bridge__$StructuredExtractionResponse$mime_type(ptr))
    }
}
extension StructuredExtractionResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StructuredExtractionResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StructuredExtractionResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StructuredExtractionResponse) {
        __swift_bridge__$Vec_StructuredExtractionResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StructuredExtractionResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredExtractionResponseRef> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredExtractionResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredExtractionResponseRefMut> {
        let pointer = __swift_bridge__$Vec_StructuredExtractionResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredExtractionResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StructuredExtractionResponseRef> {
        UnsafePointer<StructuredExtractionResponseRef>(OpaquePointer(__swift_bridge__$Vec_StructuredExtractionResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StructuredExtractionResponse$len(vecPtr)
    }
}


public class OpenWebDocumentResponse: OpenWebDocumentResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OpenWebDocumentResponse$_free(ptr)
        }
    }
}
public class OpenWebDocumentResponseRefMut: OpenWebDocumentResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OpenWebDocumentResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OpenWebDocumentResponseRef {
    public func page_content() -> RustString {
        RustString(ptr: __swift_bridge__$OpenWebDocumentResponse$page_content(ptr))
    }

    public func metadata() -> RustString {
        RustString(ptr: __swift_bridge__$OpenWebDocumentResponse$metadata(ptr))
    }
}
extension OpenWebDocumentResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OpenWebDocumentResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OpenWebDocumentResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OpenWebDocumentResponse) {
        __swift_bridge__$Vec_OpenWebDocumentResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OpenWebDocumentResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OpenWebDocumentResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OpenWebDocumentResponseRef> {
        let pointer = __swift_bridge__$Vec_OpenWebDocumentResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OpenWebDocumentResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OpenWebDocumentResponseRefMut> {
        let pointer = __swift_bridge__$Vec_OpenWebDocumentResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OpenWebDocumentResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OpenWebDocumentResponseRef> {
        UnsafePointer<OpenWebDocumentResponseRef>(OpaquePointer(__swift_bridge__$Vec_OpenWebDocumentResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OpenWebDocumentResponse$len(vecPtr)
    }
}


public class DoclingCompatResponse: DoclingCompatResponseRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DoclingCompatResponse$_free(ptr)
        }
    }
}
public class DoclingCompatResponseRefMut: DoclingCompatResponseRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DoclingCompatResponseRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DoclingCompatResponseRef {
    public func document() -> RustString {
        RustString(ptr: __swift_bridge__$DoclingCompatResponse$document(ptr))
    }

    public func status() -> RustString {
        RustString(ptr: __swift_bridge__$DoclingCompatResponse$status(ptr))
    }
}
extension DoclingCompatResponse: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DoclingCompatResponse$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DoclingCompatResponse$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DoclingCompatResponse) {
        __swift_bridge__$Vec_DoclingCompatResponse$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DoclingCompatResponse$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DoclingCompatResponse(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DoclingCompatResponseRef> {
        let pointer = __swift_bridge__$Vec_DoclingCompatResponse$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DoclingCompatResponseRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DoclingCompatResponseRefMut> {
        let pointer = __swift_bridge__$Vec_DoclingCompatResponse$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DoclingCompatResponseRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DoclingCompatResponseRef> {
        UnsafePointer<DoclingCompatResponseRef>(OpaquePointer(__swift_bridge__$Vec_DoclingCompatResponse$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DoclingCompatResponse$len(vecPtr)
    }
}


public class DetectMimeTypeParams: DetectMimeTypeParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DetectMimeTypeParams$_free(ptr)
        }
    }
}
public class DetectMimeTypeParamsRefMut: DetectMimeTypeParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DetectMimeTypeParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DetectMimeTypeParamsRef {
    public func path() -> RustString {
        RustString(ptr: __swift_bridge__$DetectMimeTypeParams$path(ptr))
    }

    public func use_content() -> Bool {
        __swift_bridge__$DetectMimeTypeParams$use_content(ptr)
    }
}
extension DetectMimeTypeParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DetectMimeTypeParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DetectMimeTypeParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DetectMimeTypeParams) {
        __swift_bridge__$Vec_DetectMimeTypeParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DetectMimeTypeParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DetectMimeTypeParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectMimeTypeParamsRef> {
        let pointer = __swift_bridge__$Vec_DetectMimeTypeParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectMimeTypeParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectMimeTypeParamsRefMut> {
        let pointer = __swift_bridge__$Vec_DetectMimeTypeParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectMimeTypeParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DetectMimeTypeParamsRef> {
        UnsafePointer<DetectMimeTypeParamsRef>(OpaquePointer(__swift_bridge__$Vec_DetectMimeTypeParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DetectMimeTypeParams$len(vecPtr)
    }
}


public class CacheWarmParams: CacheWarmParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CacheWarmParams$_free(ptr)
        }
    }
}
public class CacheWarmParamsRefMut: CacheWarmParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CacheWarmParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CacheWarmParamsRef {
    public func all_embeddings() -> Bool {
        __swift_bridge__$CacheWarmParams$all_embeddings(ptr)
    }

    public func embedding_model() -> Optional<RustString> {
        { let val = __swift_bridge__$CacheWarmParams$embedding_model(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension CacheWarmParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CacheWarmParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CacheWarmParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CacheWarmParams) {
        __swift_bridge__$Vec_CacheWarmParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CacheWarmParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CacheWarmParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CacheWarmParamsRef> {
        let pointer = __swift_bridge__$Vec_CacheWarmParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CacheWarmParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CacheWarmParamsRefMut> {
        let pointer = __swift_bridge__$Vec_CacheWarmParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CacheWarmParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CacheWarmParamsRef> {
        UnsafePointer<CacheWarmParamsRef>(OpaquePointer(__swift_bridge__$Vec_CacheWarmParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CacheWarmParams$len(vecPtr)
    }
}


public class EmbedTextParams: EmbedTextParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbedTextParams$_free(ptr)
        }
    }
}
public class EmbedTextParamsRefMut: EmbedTextParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbedTextParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbedTextParamsRef {
    public func texts() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$EmbedTextParams$texts(ptr))
    }

    public func preset() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbedTextParams$preset(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func model() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbedTextParams$model(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func api_key() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbedTextParams$api_key(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func embedding_plugin() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbedTextParams$embedding_plugin(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension EmbedTextParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbedTextParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbedTextParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbedTextParams) {
        __swift_bridge__$Vec_EmbedTextParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbedTextParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbedTextParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedTextParamsRef> {
        let pointer = __swift_bridge__$Vec_EmbedTextParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedTextParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbedTextParamsRefMut> {
        let pointer = __swift_bridge__$Vec_EmbedTextParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbedTextParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbedTextParamsRef> {
        UnsafePointer<EmbedTextParamsRef>(OpaquePointer(__swift_bridge__$Vec_EmbedTextParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbedTextParams$len(vecPtr)
    }
}


public class ExtractStructuredParams: ExtractStructuredParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractStructuredParams$_free(ptr)
        }
    }
}
public class ExtractStructuredParamsRefMut: ExtractStructuredParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractStructuredParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractStructuredParamsRef {
    public func path() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractStructuredParams$path(ptr))
    }

    public func schema() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractStructuredParams$schema(ptr))
    }

    public func model() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractStructuredParams$model(ptr))
    }

    public func schema_name() -> RustString {
        RustString(ptr: __swift_bridge__$ExtractStructuredParams$schema_name(ptr))
    }

    public func schema_description() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractStructuredParams$schema_description(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func prompt() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractStructuredParams$prompt(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func api_key() -> Optional<RustString> {
        { let val = __swift_bridge__$ExtractStructuredParams$api_key(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func strict() -> Bool {
        __swift_bridge__$ExtractStructuredParams$strict(ptr)
    }
}
extension ExtractStructuredParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractStructuredParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractStructuredParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractStructuredParams) {
        __swift_bridge__$Vec_ExtractStructuredParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractStructuredParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractStructuredParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractStructuredParamsRef> {
        let pointer = __swift_bridge__$Vec_ExtractStructuredParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractStructuredParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractStructuredParamsRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractStructuredParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractStructuredParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractStructuredParamsRef> {
        UnsafePointer<ExtractStructuredParamsRef>(OpaquePointer(__swift_bridge__$Vec_ExtractStructuredParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractStructuredParams$len(vecPtr)
    }
}


public class ChunkTextParams: ChunkTextParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkTextParams$_free(ptr)
        }
    }
}
public class ChunkTextParamsRefMut: ChunkTextParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkTextParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkTextParamsRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$ChunkTextParams$text(ptr))
    }

    public func max_characters() -> Optional<UInt> {
        __swift_bridge__$ChunkTextParams$max_characters(ptr).intoSwiftRepr()
    }

    public func overlap() -> Optional<UInt> {
        __swift_bridge__$ChunkTextParams$overlap(ptr).intoSwiftRepr()
    }

    public func chunker_type() -> Optional<RustString> {
        { let val = __swift_bridge__$ChunkTextParams$chunker_type(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func topic_threshold() -> Optional<Float> {
        __swift_bridge__$ChunkTextParams$topic_threshold(ptr).intoSwiftRepr()
    }
}
extension ChunkTextParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkTextParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkTextParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkTextParams) {
        __swift_bridge__$Vec_ChunkTextParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkTextParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkTextParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkTextParamsRef> {
        let pointer = __swift_bridge__$Vec_ChunkTextParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkTextParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkTextParamsRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkTextParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkTextParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkTextParamsRef> {
        UnsafePointer<ChunkTextParamsRef>(OpaquePointer(__swift_bridge__$Vec_ChunkTextParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkTextParams$len(vecPtr)
    }
}


public class DetectedBoundary: DetectedBoundaryRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DetectedBoundary$_free(ptr)
        }
    }
}
public class DetectedBoundaryRefMut: DetectedBoundaryRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DetectedBoundaryRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DetectedBoundaryRef {
    public func byte_offset() -> UInt {
        __swift_bridge__$DetectedBoundary$byte_offset(ptr)
    }

    public func is_header() -> Bool {
        __swift_bridge__$DetectedBoundary$is_header(ptr)
    }
}
extension DetectedBoundary: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DetectedBoundary$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DetectedBoundary$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DetectedBoundary) {
        __swift_bridge__$Vec_DetectedBoundary$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DetectedBoundary$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DetectedBoundary(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectedBoundaryRef> {
        let pointer = __swift_bridge__$Vec_DetectedBoundary$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectedBoundaryRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectedBoundaryRefMut> {
        let pointer = __swift_bridge__$Vec_DetectedBoundary$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectedBoundaryRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DetectedBoundaryRef> {
        UnsafePointer<DetectedBoundaryRef>(OpaquePointer(__swift_bridge__$Vec_DetectedBoundary$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DetectedBoundary$len(vecPtr)
    }
}


public class ChunkingResult: ChunkingResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkingResult$_free(ptr)
        }
    }
}
public class ChunkingResultRefMut: ChunkingResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkingResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkingResultRef {
    public func chunks() -> RustVec<Chunk> {
        RustVec(ptr: __swift_bridge__$ChunkingResult$chunks(ptr))
    }

    public func chunk_count() -> UInt {
        __swift_bridge__$ChunkingResult$chunk_count(ptr)
    }
}
extension ChunkingResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkingResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkingResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkingResult) {
        __swift_bridge__$Vec_ChunkingResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkingResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkingResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkingResultRef> {
        let pointer = __swift_bridge__$Vec_ChunkingResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkingResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkingResultRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkingResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkingResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkingResultRef> {
        UnsafePointer<ChunkingResultRef>(OpaquePointer(__swift_bridge__$Vec_ChunkingResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkingResult$len(vecPtr)
    }
}


public class MergedChunk: MergedChunkRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$MergedChunk$_free(ptr)
        }
    }
}
public class MergedChunkRefMut: MergedChunkRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class MergedChunkRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension MergedChunkRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$MergedChunk$text(ptr))
    }

    public func byte_start() -> UInt {
        __swift_bridge__$MergedChunk$byte_start(ptr)
    }

    public func byte_end() -> UInt {
        __swift_bridge__$MergedChunk$byte_end(ptr)
    }
}
extension MergedChunk: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_MergedChunk$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_MergedChunk$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: MergedChunk) {
        __swift_bridge__$Vec_MergedChunk$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_MergedChunk$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (MergedChunk(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<MergedChunkRef> {
        let pointer = __swift_bridge__$Vec_MergedChunk$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return MergedChunkRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<MergedChunkRefMut> {
        let pointer = __swift_bridge__$Vec_MergedChunk$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return MergedChunkRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<MergedChunkRef> {
        UnsafePointer<MergedChunkRef>(OpaquePointer(__swift_bridge__$Vec_MergedChunk$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_MergedChunk$len(vecPtr)
    }
}


public class EmbeddingPreset: EmbeddingPresetRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbeddingPreset$_free(ptr)
        }
    }
}
public class EmbeddingPresetRefMut: EmbeddingPresetRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbeddingPresetRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbeddingPresetRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddingPreset$name(ptr))
    }

    public func chunk_size() -> UInt {
        __swift_bridge__$EmbeddingPreset$chunk_size(ptr)
    }

    public func overlap() -> UInt {
        __swift_bridge__$EmbeddingPreset$overlap(ptr)
    }

    public func model_repo() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddingPreset$model_repo(ptr))
    }

    public func pooling() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddingPreset$pooling(ptr))
    }

    public func model_file() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddingPreset$model_file(ptr))
    }

    public func dimensions() -> UInt {
        __swift_bridge__$EmbeddingPreset$dimensions(ptr)
    }

    public func description() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddingPreset$description(ptr))
    }
}
extension EmbeddingPreset: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbeddingPreset$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbeddingPreset$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbeddingPreset) {
        __swift_bridge__$Vec_EmbeddingPreset$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbeddingPreset$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbeddingPreset(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingPresetRef> {
        let pointer = __swift_bridge__$Vec_EmbeddingPreset$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingPresetRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingPresetRefMut> {
        let pointer = __swift_bridge__$Vec_EmbeddingPreset$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingPresetRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbeddingPresetRef> {
        UnsafePointer<EmbeddingPresetRef>(OpaquePointer(__swift_bridge__$Vec_EmbeddingPreset$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbeddingPreset$len(vecPtr)
    }
}


public class YakeParams: YakeParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$YakeParams$_free(ptr)
        }
    }
}
extension YakeParams {
    public convenience init(_ window_size: UInt) {
        self.init(ptr: __swift_bridge__$YakeParams$new(window_size))
    }
}
public class YakeParamsRefMut: YakeParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class YakeParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension YakeParamsRef {
    public func window_size() -> UInt {
        __swift_bridge__$YakeParams$window_size(ptr)
    }
}
extension YakeParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_YakeParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_YakeParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: YakeParams) {
        __swift_bridge__$Vec_YakeParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_YakeParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (YakeParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<YakeParamsRef> {
        let pointer = __swift_bridge__$Vec_YakeParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return YakeParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<YakeParamsRefMut> {
        let pointer = __swift_bridge__$Vec_YakeParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return YakeParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<YakeParamsRef> {
        UnsafePointer<YakeParamsRef>(OpaquePointer(__swift_bridge__$Vec_YakeParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_YakeParams$len(vecPtr)
    }
}


public class RakeParams: RakeParamsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RakeParams$_free(ptr)
        }
    }
}
extension RakeParams {
    public convenience init(_ min_word_length: UInt, _ max_words_per_phrase: UInt) {
        self.init(ptr: __swift_bridge__$RakeParams$new(min_word_length, max_words_per_phrase))
    }
}
public class RakeParamsRefMut: RakeParamsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RakeParamsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RakeParamsRef {
    public func min_word_length() -> UInt {
        __swift_bridge__$RakeParams$min_word_length(ptr)
    }

    public func max_words_per_phrase() -> UInt {
        __swift_bridge__$RakeParams$max_words_per_phrase(ptr)
    }
}
extension RakeParams: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RakeParams$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RakeParams$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RakeParams) {
        __swift_bridge__$Vec_RakeParams$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RakeParams$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RakeParams(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RakeParamsRef> {
        let pointer = __swift_bridge__$Vec_RakeParams$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RakeParamsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RakeParamsRefMut> {
        let pointer = __swift_bridge__$Vec_RakeParams$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RakeParamsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RakeParamsRef> {
        UnsafePointer<RakeParamsRef>(OpaquePointer(__swift_bridge__$Vec_RakeParams$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RakeParams$len(vecPtr)
    }
}


public class KeywordConfig: KeywordConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$KeywordConfig$_free(ptr)
        }
    }
}
extension KeywordConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ algorithm: KeywordAlgorithm, _ max_keywords: UInt, _ min_score: Float, _ ngram_range: RustVec<UInt>, _ language: Optional<GenericIntoRustString>, _ yake_params: Optional<YakeParams>, _ rake_params: Optional<RakeParams>) {
        self.init(ptr: __swift_bridge__$KeywordConfig$new({algorithm.isOwned = false; return algorithm.ptr;}(), max_keywords, min_score, { let val = ngram_range; val.isOwned = false; return val.ptr }(), { if let rustString = optionalStringIntoRustString(language) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let val = yake_params { val.isOwned = false; return val.ptr } else { return nil } }(), { if let val = rake_params { val.isOwned = false; return val.ptr } else { return nil } }()))
    }
}
public class KeywordConfigRefMut: KeywordConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class KeywordConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension KeywordConfigRef {
    public func algorithm() -> KeywordAlgorithm {
        KeywordAlgorithm(ptr: __swift_bridge__$KeywordConfig$algorithm(ptr))
    }

    public func max_keywords() -> UInt {
        __swift_bridge__$KeywordConfig$max_keywords(ptr)
    }

    public func min_score() -> Float {
        __swift_bridge__$KeywordConfig$min_score(ptr)
    }

    public func ngram_range() -> RustVec<UInt> {
        RustVec(ptr: __swift_bridge__$KeywordConfig$ngram_range(ptr))
    }

    public func language() -> Optional<RustString> {
        { let val = __swift_bridge__$KeywordConfig$language(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func yake_params() -> Optional<YakeParams> {
        { let val = __swift_bridge__$KeywordConfig$yake_params(ptr); if val != nil { return YakeParams(ptr: val!) } else { return nil } }()
    }

    public func rake_params() -> Optional<RakeParams> {
        { let val = __swift_bridge__$KeywordConfig$rake_params(ptr); if val != nil { return RakeParams(ptr: val!) } else { return nil } }()
    }
}
extension KeywordConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_KeywordConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_KeywordConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: KeywordConfig) {
        __swift_bridge__$Vec_KeywordConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_KeywordConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (KeywordConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordConfigRef> {
        let pointer = __swift_bridge__$Vec_KeywordConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordConfigRefMut> {
        let pointer = __swift_bridge__$Vec_KeywordConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<KeywordConfigRef> {
        UnsafePointer<KeywordConfigRef>(OpaquePointer(__swift_bridge__$Vec_KeywordConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_KeywordConfig$len(vecPtr)
    }
}


public class Keyword: KeywordRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Keyword$_free(ptr)
        }
    }
}
public class KeywordRefMut: KeywordRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class KeywordRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension KeywordRef {
    public func text() -> RustString {
        RustString(ptr: __swift_bridge__$Keyword$text(ptr))
    }

    public func score() -> Float {
        __swift_bridge__$Keyword$score(ptr)
    }

    public func algorithm() -> KeywordAlgorithm {
        KeywordAlgorithm(ptr: __swift_bridge__$Keyword$algorithm(ptr))
    }

    public func positions() -> Optional<RustVec<UInt>> {
        { let val = __swift_bridge__$Keyword$positions(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }
}
extension Keyword: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Keyword$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Keyword$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Keyword) {
        __swift_bridge__$Vec_Keyword$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Keyword$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Keyword(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordRef> {
        let pointer = __swift_bridge__$Vec_Keyword$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordRefMut> {
        let pointer = __swift_bridge__$Vec_Keyword$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<KeywordRef> {
        UnsafePointer<KeywordRef>(OpaquePointer(__swift_bridge__$Vec_Keyword$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Keyword$len(vecPtr)
    }
}


public class OcrCacheStats: OcrCacheStatsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrCacheStats$_free(ptr)
        }
    }
}
extension OcrCacheStats {
    public convenience init(_ total_files: UInt, _ total_size_mb: Double) {
        self.init(ptr: __swift_bridge__$OcrCacheStats$new(total_files, total_size_mb))
    }
}
public class OcrCacheStatsRefMut: OcrCacheStatsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrCacheStatsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrCacheStatsRef {
    public func total_files() -> UInt {
        __swift_bridge__$OcrCacheStats$total_files(ptr)
    }

    public func total_size_mb() -> Double {
        __swift_bridge__$OcrCacheStats$total_size_mb(ptr)
    }
}
extension OcrCacheStats: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrCacheStats$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrCacheStats$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrCacheStats) {
        __swift_bridge__$Vec_OcrCacheStats$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrCacheStats$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrCacheStats(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrCacheStatsRef> {
        let pointer = __swift_bridge__$Vec_OcrCacheStats$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrCacheStatsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrCacheStatsRefMut> {
        let pointer = __swift_bridge__$Vec_OcrCacheStats$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrCacheStatsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrCacheStatsRef> {
        UnsafePointer<OcrCacheStatsRef>(OpaquePointer(__swift_bridge__$Vec_OcrCacheStats$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrCacheStats$len(vecPtr)
    }
}


public class RecognizedTable: RecognizedTableRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RecognizedTable$_free(ptr)
        }
    }
}
public class RecognizedTableRefMut: RecognizedTableRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RecognizedTableRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RecognizedTableRef {
    public func detection_bbox() -> BBox {
        BBox(ptr: __swift_bridge__$RecognizedTable$detection_bbox(ptr))
    }

    public func cells() -> RustString {
        RustString(ptr: __swift_bridge__$RecognizedTable$cells(ptr))
    }

    public func markdown() -> RustString {
        RustString(ptr: __swift_bridge__$RecognizedTable$markdown(ptr))
    }
}
extension RecognizedTable: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RecognizedTable$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RecognizedTable$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RecognizedTable) {
        __swift_bridge__$Vec_RecognizedTable$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RecognizedTable$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RecognizedTable(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RecognizedTableRef> {
        let pointer = __swift_bridge__$Vec_RecognizedTable$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RecognizedTableRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RecognizedTableRefMut> {
        let pointer = __swift_bridge__$Vec_RecognizedTable$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RecognizedTableRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RecognizedTableRef> {
        UnsafePointer<RecognizedTableRef>(OpaquePointer(__swift_bridge__$Vec_RecognizedTable$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RecognizedTable$len(vecPtr)
    }
}


public class TessdataManager: TessdataManagerRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TessdataManager$_free(ptr)
        }
    }
}
public class TessdataManagerRefMut: TessdataManagerRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TessdataManagerRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TessdataManager: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TessdataManager$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TessdataManager$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TessdataManager) {
        __swift_bridge__$Vec_TessdataManager$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TessdataManager$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TessdataManager(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TessdataManagerRef> {
        let pointer = __swift_bridge__$Vec_TessdataManager$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TessdataManagerRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TessdataManagerRefMut> {
        let pointer = __swift_bridge__$Vec_TessdataManager$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TessdataManagerRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TessdataManagerRef> {
        UnsafePointer<TessdataManagerRef>(OpaquePointer(__swift_bridge__$Vec_TessdataManager$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TessdataManager$len(vecPtr)
    }
}


public class PaddleOcrConfig: PaddleOcrConfigRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PaddleOcrConfig$_free(ptr)
        }
    }
}
extension PaddleOcrConfig {
    public convenience init<GenericIntoRustString: IntoRustString>(_ language: GenericIntoRustString, _ cache_dir: Optional<GenericIntoRustString>, _ use_angle_cls: Bool, _ enable_table_detection: Bool, _ det_db_thresh: Float, _ det_db_box_thresh: Float, _ det_db_unclip_ratio: Float, _ det_limit_side_len: UInt32, _ rec_batch_num: UInt32, _ padding: UInt32, _ drop_score: Float, _ model_tier: GenericIntoRustString) {
        self.init(ptr: __swift_bridge__$PaddleOcrConfig$new({ let rustString = language.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { if let rustString = optionalStringIntoRustString(cache_dir) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), use_angle_cls, enable_table_detection, det_db_thresh, det_db_box_thresh, det_db_unclip_ratio, det_limit_side_len, rec_batch_num, padding, drop_score, { let rustString = model_tier.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }
}
public class PaddleOcrConfigRefMut: PaddleOcrConfigRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PaddleOcrConfigRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PaddleOcrConfigRef {
    public func language() -> RustString {
        RustString(ptr: __swift_bridge__$PaddleOcrConfig$language(ptr))
    }

    public func cache_dir() -> Optional<RustString> {
        { let val = __swift_bridge__$PaddleOcrConfig$cache_dir(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func use_angle_cls() -> Bool {
        __swift_bridge__$PaddleOcrConfig$use_angle_cls(ptr)
    }

    public func enable_table_detection() -> Bool {
        __swift_bridge__$PaddleOcrConfig$enable_table_detection(ptr)
    }

    public func det_db_thresh() -> Float {
        __swift_bridge__$PaddleOcrConfig$det_db_thresh(ptr)
    }

    public func det_db_box_thresh() -> Float {
        __swift_bridge__$PaddleOcrConfig$det_db_box_thresh(ptr)
    }

    public func det_db_unclip_ratio() -> Float {
        __swift_bridge__$PaddleOcrConfig$det_db_unclip_ratio(ptr)
    }

    public func det_limit_side_len() -> UInt32 {
        __swift_bridge__$PaddleOcrConfig$det_limit_side_len(ptr)
    }

    public func rec_batch_num() -> UInt32 {
        __swift_bridge__$PaddleOcrConfig$rec_batch_num(ptr)
    }

    public func padding() -> UInt32 {
        __swift_bridge__$PaddleOcrConfig$padding(ptr)
    }

    public func drop_score() -> Float {
        __swift_bridge__$PaddleOcrConfig$drop_score(ptr)
    }

    public func model_tier() -> RustString {
        RustString(ptr: __swift_bridge__$PaddleOcrConfig$model_tier(ptr))
    }
}
extension PaddleOcrConfig: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PaddleOcrConfig$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PaddleOcrConfig$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PaddleOcrConfig) {
        __swift_bridge__$Vec_PaddleOcrConfig$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PaddleOcrConfig$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PaddleOcrConfig(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PaddleOcrConfigRef> {
        let pointer = __swift_bridge__$Vec_PaddleOcrConfig$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PaddleOcrConfigRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PaddleOcrConfigRefMut> {
        let pointer = __swift_bridge__$Vec_PaddleOcrConfig$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PaddleOcrConfigRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PaddleOcrConfigRef> {
        UnsafePointer<PaddleOcrConfigRef>(OpaquePointer(__swift_bridge__$Vec_PaddleOcrConfig$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PaddleOcrConfig$len(vecPtr)
    }
}


public class ModelPaths: ModelPathsRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ModelPaths$_free(ptr)
        }
    }
}
public class ModelPathsRefMut: ModelPathsRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ModelPathsRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ModelPathsRef {
    public func det_model() -> RustString {
        RustString(ptr: __swift_bridge__$ModelPaths$det_model(ptr))
    }

    public func cls_model() -> RustString {
        RustString(ptr: __swift_bridge__$ModelPaths$cls_model(ptr))
    }

    public func rec_model() -> RustString {
        RustString(ptr: __swift_bridge__$ModelPaths$rec_model(ptr))
    }

    public func dict_file() -> RustString {
        RustString(ptr: __swift_bridge__$ModelPaths$dict_file(ptr))
    }
}
extension ModelPaths: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ModelPaths$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ModelPaths$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ModelPaths) {
        __swift_bridge__$Vec_ModelPaths$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ModelPaths$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ModelPaths(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ModelPathsRef> {
        let pointer = __swift_bridge__$Vec_ModelPaths$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ModelPathsRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ModelPathsRefMut> {
        let pointer = __swift_bridge__$Vec_ModelPaths$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ModelPathsRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ModelPathsRef> {
        UnsafePointer<ModelPathsRef>(OpaquePointer(__swift_bridge__$Vec_ModelPaths$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ModelPaths$len(vecPtr)
    }
}


public class OrientationResult: OrientationResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OrientationResult$_free(ptr)
        }
    }
}
public class OrientationResultRefMut: OrientationResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OrientationResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OrientationResultRef {
    public func degrees() -> UInt32 {
        __swift_bridge__$OrientationResult$degrees(ptr)
    }

    public func confidence() -> Float {
        __swift_bridge__$OrientationResult$confidence(ptr)
    }
}
extension OrientationResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OrientationResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OrientationResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OrientationResult) {
        __swift_bridge__$Vec_OrientationResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OrientationResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OrientationResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OrientationResultRef> {
        let pointer = __swift_bridge__$Vec_OrientationResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OrientationResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OrientationResultRefMut> {
        let pointer = __swift_bridge__$Vec_OrientationResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OrientationResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OrientationResultRef> {
        UnsafePointer<OrientationResultRef>(OpaquePointer(__swift_bridge__$Vec_OrientationResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OrientationResult$len(vecPtr)
    }
}


public class BBox: BBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$BBox$_free(ptr)
        }
    }
}
public class BBoxRefMut: BBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class BBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension BBoxRef {
    public func x1() -> Float {
        __swift_bridge__$BBox$x1(ptr)
    }

    public func y1() -> Float {
        __swift_bridge__$BBox$y1(ptr)
    }

    public func x2() -> Float {
        __swift_bridge__$BBox$x2(ptr)
    }

    public func y2() -> Float {
        __swift_bridge__$BBox$y2(ptr)
    }
}
extension BBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_BBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_BBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: BBox) {
        __swift_bridge__$Vec_BBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_BBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (BBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BBoxRef> {
        let pointer = __swift_bridge__$Vec_BBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BBoxRefMut> {
        let pointer = __swift_bridge__$Vec_BBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<BBoxRef> {
        UnsafePointer<BBoxRef>(OpaquePointer(__swift_bridge__$Vec_BBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_BBox$len(vecPtr)
    }
}


public class LayoutDetection: LayoutDetectionRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LayoutDetection$_free(ptr)
        }
    }
}
public class LayoutDetectionRefMut: LayoutDetectionRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LayoutDetectionRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LayoutDetectionRef {
    public func class_name() -> LayoutClass {
        LayoutClass(ptr: __swift_bridge__$LayoutDetection$class_name(ptr))
    }

    public func confidence() -> Float {
        __swift_bridge__$LayoutDetection$confidence(ptr)
    }

    public func bbox() -> BBox {
        BBox(ptr: __swift_bridge__$LayoutDetection$bbox(ptr))
    }
}
extension LayoutDetection: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LayoutDetection$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LayoutDetection$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LayoutDetection) {
        __swift_bridge__$Vec_LayoutDetection$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LayoutDetection$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LayoutDetection(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutDetectionRef> {
        let pointer = __swift_bridge__$Vec_LayoutDetection$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutDetectionRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutDetectionRefMut> {
        let pointer = __swift_bridge__$Vec_LayoutDetection$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutDetectionRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LayoutDetectionRef> {
        UnsafePointer<LayoutDetectionRef>(OpaquePointer(__swift_bridge__$Vec_LayoutDetection$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LayoutDetection$len(vecPtr)
    }
}


public class DetectionResult: DetectionResultRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$DetectionResult$_free(ptr)
        }
    }
}
public class DetectionResultRefMut: DetectionResultRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class DetectionResultRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension DetectionResultRef {
    public func page_width() -> UInt32 {
        __swift_bridge__$DetectionResult$page_width(ptr)
    }

    public func page_height() -> UInt32 {
        __swift_bridge__$DetectionResult$page_height(ptr)
    }

    public func detections() -> RustVec<LayoutDetection> {
        RustVec(ptr: __swift_bridge__$DetectionResult$detections(ptr))
    }
}
extension DetectionResult: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_DetectionResult$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_DetectionResult$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: DetectionResult) {
        __swift_bridge__$Vec_DetectionResult$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_DetectionResult$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (DetectionResult(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectionResultRef> {
        let pointer = __swift_bridge__$Vec_DetectionResult$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectionResultRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<DetectionResultRefMut> {
        let pointer = __swift_bridge__$Vec_DetectionResult$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return DetectionResultRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<DetectionResultRef> {
        UnsafePointer<DetectionResultRef>(OpaquePointer(__swift_bridge__$Vec_DetectionResult$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_DetectionResult$len(vecPtr)
    }
}


public class EmbeddedFile: EmbeddedFileRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbeddedFile$_free(ptr)
        }
    }
}
public class EmbeddedFileRefMut: EmbeddedFileRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbeddedFileRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbeddedFileRef {
    public func name() -> RustString {
        RustString(ptr: __swift_bridge__$EmbeddedFile$name(ptr))
    }

    public func data() -> RustVec<UInt8> {
        RustVec(ptr: __swift_bridge__$EmbeddedFile$data(ptr))
    }

    public func mime_type() -> Optional<RustString> {
        { let val = __swift_bridge__$EmbeddedFile$mime_type(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }
}
extension EmbeddedFile: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbeddedFile$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbeddedFile$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbeddedFile) {
        __swift_bridge__$Vec_EmbeddedFile$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbeddedFile$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbeddedFile(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddedFileRef> {
        let pointer = __swift_bridge__$Vec_EmbeddedFile$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddedFileRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddedFileRefMut> {
        let pointer = __swift_bridge__$Vec_EmbeddedFile$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddedFileRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbeddedFileRef> {
        UnsafePointer<EmbeddedFileRef>(OpaquePointer(__swift_bridge__$Vec_EmbeddedFile$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbeddedFile$len(vecPtr)
    }
}


public class PdfMetadata: PdfMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PdfMetadata$_free(ptr)
        }
    }
}
extension PdfMetadata {
    public convenience init<GenericIntoRustString: IntoRustString>(_ pdf_version: Optional<GenericIntoRustString>, _ producer: Optional<GenericIntoRustString>, _ is_encrypted: Optional<Bool>, _ width: Optional<Int64>, _ height: Optional<Int64>, _ page_count: Optional<UInt>) {
        self.init(ptr: __swift_bridge__$PdfMetadata$new({ if let rustString = optionalStringIntoRustString(pdf_version) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), { if let rustString = optionalStringIntoRustString(producer) { rustString.isOwned = false; return rustString.ptr } else { return nil } }(), is_encrypted.intoFfiRepr(), width.intoFfiRepr(), height.intoFfiRepr(), page_count.intoFfiRepr()))
    }
}
public class PdfMetadataRefMut: PdfMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PdfMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PdfMetadataRef {
    public func pdf_version() -> Optional<RustString> {
        { let val = __swift_bridge__$PdfMetadata$pdf_version(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func producer() -> Optional<RustString> {
        { let val = __swift_bridge__$PdfMetadata$producer(ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
    }

    public func is_encrypted() -> Optional<Bool> {
        __swift_bridge__$PdfMetadata$is_encrypted(ptr).intoSwiftRepr()
    }

    public func width() -> Optional<Int64> {
        __swift_bridge__$PdfMetadata$width(ptr).intoSwiftRepr()
    }

    public func height() -> Optional<Int64> {
        __swift_bridge__$PdfMetadata$height(ptr).intoSwiftRepr()
    }

    public func page_count() -> Optional<UInt> {
        __swift_bridge__$PdfMetadata$page_count(ptr).intoSwiftRepr()
    }
}
extension PdfMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PdfMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PdfMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PdfMetadata) {
        __swift_bridge__$Vec_PdfMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PdfMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PdfMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfMetadataRef> {
        let pointer = __swift_bridge__$Vec_PdfMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_PdfMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PdfMetadataRef> {
        UnsafePointer<PdfMetadataRef>(OpaquePointer(__swift_bridge__$Vec_PdfMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PdfMetadata$len(vecPtr)
    }
}


public class ExecutionProviderType: ExecutionProviderTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExecutionProviderType$_free(ptr)
        }
    }
}
public class ExecutionProviderTypeRefMut: ExecutionProviderTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExecutionProviderTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExecutionProviderType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExecutionProviderType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExecutionProviderType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExecutionProviderType) {
        __swift_bridge__$Vec_ExecutionProviderType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExecutionProviderType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExecutionProviderType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExecutionProviderTypeRef> {
        let pointer = __swift_bridge__$Vec_ExecutionProviderType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExecutionProviderTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExecutionProviderTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ExecutionProviderType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExecutionProviderTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExecutionProviderTypeRef> {
        UnsafePointer<ExecutionProviderTypeRef>(OpaquePointer(__swift_bridge__$Vec_ExecutionProviderType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExecutionProviderType$len(vecPtr)
    }
}


public class OutputFormat: OutputFormatRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OutputFormat$_free(ptr)
        }
    }
}
public class OutputFormatRefMut: OutputFormatRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OutputFormatRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OutputFormat: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OutputFormat$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OutputFormat$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OutputFormat) {
        __swift_bridge__$Vec_OutputFormat$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OutputFormat$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OutputFormat(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OutputFormatRef> {
        let pointer = __swift_bridge__$Vec_OutputFormat$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OutputFormatRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OutputFormatRefMut> {
        let pointer = __swift_bridge__$Vec_OutputFormat$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OutputFormatRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OutputFormatRef> {
        UnsafePointer<OutputFormatRef>(OpaquePointer(__swift_bridge__$Vec_OutputFormat$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OutputFormat$len(vecPtr)
    }
}


public class HtmlTheme: HtmlThemeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$HtmlTheme$_free(ptr)
        }
    }
}
public class HtmlThemeRefMut: HtmlThemeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class HtmlThemeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension HtmlTheme: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_HtmlTheme$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_HtmlTheme$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: HtmlTheme) {
        __swift_bridge__$Vec_HtmlTheme$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_HtmlTheme$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (HtmlTheme(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlThemeRef> {
        let pointer = __swift_bridge__$Vec_HtmlTheme$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlThemeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<HtmlThemeRefMut> {
        let pointer = __swift_bridge__$Vec_HtmlTheme$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return HtmlThemeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<HtmlThemeRef> {
        UnsafePointer<HtmlThemeRef>(OpaquePointer(__swift_bridge__$Vec_HtmlTheme$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_HtmlTheme$len(vecPtr)
    }
}


public class TableModel: TableModelRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TableModel$_free(ptr)
        }
    }
}
public class TableModelRefMut: TableModelRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TableModelRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TableModel: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TableModel$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TableModel$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TableModel) {
        __swift_bridge__$Vec_TableModel$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TableModel$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TableModel(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableModelRef> {
        let pointer = __swift_bridge__$Vec_TableModel$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableModelRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TableModelRefMut> {
        let pointer = __swift_bridge__$Vec_TableModel$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TableModelRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TableModelRef> {
        UnsafePointer<TableModelRef>(OpaquePointer(__swift_bridge__$Vec_TableModel$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TableModel$len(vecPtr)
    }
}


public class ChunkerType: ChunkerTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkerType$_free(ptr)
        }
    }
}
public class ChunkerTypeRefMut: ChunkerTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkerTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkerType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkerType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkerType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkerType) {
        __swift_bridge__$Vec_ChunkerType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkerType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkerType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkerTypeRef> {
        let pointer = __swift_bridge__$Vec_ChunkerType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkerTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkerTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkerType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkerTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkerTypeRef> {
        UnsafePointer<ChunkerTypeRef>(OpaquePointer(__swift_bridge__$Vec_ChunkerType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkerType$len(vecPtr)
    }
}


public class ChunkSizing: ChunkSizingRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkSizing$_free(ptr)
        }
    }
}
public class ChunkSizingRefMut: ChunkSizingRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkSizingRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkSizing: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkSizing$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkSizing$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkSizing) {
        __swift_bridge__$Vec_ChunkSizing$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkSizing$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkSizing(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkSizingRef> {
        let pointer = __swift_bridge__$Vec_ChunkSizing$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkSizingRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkSizingRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkSizing$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkSizingRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkSizingRef> {
        UnsafePointer<ChunkSizingRef>(OpaquePointer(__swift_bridge__$Vec_ChunkSizing$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkSizing$len(vecPtr)
    }
}


public class EmbeddingModelType: EmbeddingModelTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbeddingModelType$_free(ptr)
        }
    }
}
public class EmbeddingModelTypeRefMut: EmbeddingModelTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbeddingModelTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbeddingModelType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbeddingModelType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbeddingModelType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbeddingModelType) {
        __swift_bridge__$Vec_EmbeddingModelType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbeddingModelType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbeddingModelType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingModelTypeRef> {
        let pointer = __swift_bridge__$Vec_EmbeddingModelType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingModelTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingModelTypeRefMut> {
        let pointer = __swift_bridge__$Vec_EmbeddingModelType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingModelTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbeddingModelTypeRef> {
        UnsafePointer<EmbeddingModelTypeRef>(OpaquePointer(__swift_bridge__$Vec_EmbeddingModelType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbeddingModelType$len(vecPtr)
    }
}


public class CodeContentMode: CodeContentModeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CodeContentMode$_free(ptr)
        }
    }
}
public class CodeContentModeRefMut: CodeContentModeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CodeContentModeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CodeContentMode: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CodeContentMode$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CodeContentMode$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CodeContentMode) {
        __swift_bridge__$Vec_CodeContentMode$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CodeContentMode$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CodeContentMode(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CodeContentModeRef> {
        let pointer = __swift_bridge__$Vec_CodeContentMode$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CodeContentModeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CodeContentModeRefMut> {
        let pointer = __swift_bridge__$Vec_CodeContentMode$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CodeContentModeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CodeContentModeRef> {
        UnsafePointer<CodeContentModeRef>(OpaquePointer(__swift_bridge__$Vec_CodeContentMode$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CodeContentMode$len(vecPtr)
    }
}


public class FracType: FracTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$FracType$_free(ptr)
        }
    }
}
public class FracTypeRefMut: FracTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FracTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FracType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_FracType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_FracType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: FracType) {
        __swift_bridge__$Vec_FracType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_FracType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (FracType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FracTypeRef> {
        let pointer = __swift_bridge__$Vec_FracType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FracTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FracTypeRefMut> {
        let pointer = __swift_bridge__$Vec_FracType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FracTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FracTypeRef> {
        UnsafePointer<FracTypeRef>(OpaquePointer(__swift_bridge__$Vec_FracType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_FracType$len(vecPtr)
    }
}


public class OcrBackendType: OcrBackendTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrBackendType$_free(ptr)
        }
    }
}
public class OcrBackendTypeRefMut: OcrBackendTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrBackendTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrBackendType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrBackendType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrBackendType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrBackendType) {
        __swift_bridge__$Vec_OcrBackendType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrBackendType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrBackendType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBackendTypeRef> {
        let pointer = __swift_bridge__$Vec_OcrBackendType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBackendTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBackendTypeRefMut> {
        let pointer = __swift_bridge__$Vec_OcrBackendType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBackendTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrBackendTypeRef> {
        UnsafePointer<OcrBackendTypeRef>(OpaquePointer(__swift_bridge__$Vec_OcrBackendType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrBackendType$len(vecPtr)
    }
}


public class ProcessingStage: ProcessingStageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ProcessingStage$_free(ptr)
        }
    }
}
public class ProcessingStageRefMut: ProcessingStageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ProcessingStageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ProcessingStage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ProcessingStage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ProcessingStage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ProcessingStage) {
        __swift_bridge__$Vec_ProcessingStage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ProcessingStage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ProcessingStage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ProcessingStageRef> {
        let pointer = __swift_bridge__$Vec_ProcessingStage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ProcessingStageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ProcessingStageRefMut> {
        let pointer = __swift_bridge__$Vec_ProcessingStage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ProcessingStageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ProcessingStageRef> {
        UnsafePointer<ProcessingStageRef>(OpaquePointer(__swift_bridge__$Vec_ProcessingStage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ProcessingStage$len(vecPtr)
    }
}


public class ReductionLevel: ReductionLevelRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ReductionLevel$_free(ptr)
        }
    }
}
public class ReductionLevelRefMut: ReductionLevelRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ReductionLevelRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ReductionLevel: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ReductionLevel$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ReductionLevel$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ReductionLevel) {
        __swift_bridge__$Vec_ReductionLevel$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ReductionLevel$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ReductionLevel(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ReductionLevelRef> {
        let pointer = __swift_bridge__$Vec_ReductionLevel$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ReductionLevelRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ReductionLevelRefMut> {
        let pointer = __swift_bridge__$Vec_ReductionLevel$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ReductionLevelRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ReductionLevelRef> {
        UnsafePointer<ReductionLevelRef>(OpaquePointer(__swift_bridge__$Vec_ReductionLevel$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ReductionLevel$len(vecPtr)
    }
}


public class PdfAnnotationType: PdfAnnotationTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PdfAnnotationType$_free(ptr)
        }
    }
}
public class PdfAnnotationTypeRefMut: PdfAnnotationTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PdfAnnotationTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PdfAnnotationType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PdfAnnotationType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PdfAnnotationType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PdfAnnotationType) {
        __swift_bridge__$Vec_PdfAnnotationType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PdfAnnotationType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PdfAnnotationType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfAnnotationTypeRef> {
        let pointer = __swift_bridge__$Vec_PdfAnnotationType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfAnnotationTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PdfAnnotationTypeRefMut> {
        let pointer = __swift_bridge__$Vec_PdfAnnotationType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PdfAnnotationTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PdfAnnotationTypeRef> {
        UnsafePointer<PdfAnnotationTypeRef>(OpaquePointer(__swift_bridge__$Vec_PdfAnnotationType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PdfAnnotationType$len(vecPtr)
    }
}


public class BlockType: BlockTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$BlockType$_free(ptr)
        }
    }
}
public class BlockTypeRefMut: BlockTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class BlockTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension BlockType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_BlockType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_BlockType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: BlockType) {
        __swift_bridge__$Vec_BlockType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_BlockType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (BlockType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BlockTypeRef> {
        let pointer = __swift_bridge__$Vec_BlockType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BlockTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<BlockTypeRefMut> {
        let pointer = __swift_bridge__$Vec_BlockType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return BlockTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<BlockTypeRef> {
        UnsafePointer<BlockTypeRef>(OpaquePointer(__swift_bridge__$Vec_BlockType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_BlockType$len(vecPtr)
    }
}


public class InlineType: InlineTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$InlineType$_free(ptr)
        }
    }
}
public class InlineTypeRefMut: InlineTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class InlineTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension InlineType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_InlineType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_InlineType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: InlineType) {
        __swift_bridge__$Vec_InlineType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_InlineType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (InlineType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InlineTypeRef> {
        let pointer = __swift_bridge__$Vec_InlineType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InlineTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InlineTypeRefMut> {
        let pointer = __swift_bridge__$Vec_InlineType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InlineTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<InlineTypeRef> {
        UnsafePointer<InlineTypeRef>(OpaquePointer(__swift_bridge__$Vec_InlineType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_InlineType$len(vecPtr)
    }
}


public class RelationshipKind: RelationshipKindRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RelationshipKind$_free(ptr)
        }
    }
}
public class RelationshipKindRefMut: RelationshipKindRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RelationshipKindRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RelationshipKind: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RelationshipKind$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RelationshipKind$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RelationshipKind) {
        __swift_bridge__$Vec_RelationshipKind$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RelationshipKind$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RelationshipKind(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RelationshipKindRef> {
        let pointer = __swift_bridge__$Vec_RelationshipKind$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RelationshipKindRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RelationshipKindRefMut> {
        let pointer = __swift_bridge__$Vec_RelationshipKind$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RelationshipKindRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RelationshipKindRef> {
        UnsafePointer<RelationshipKindRef>(OpaquePointer(__swift_bridge__$Vec_RelationshipKind$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RelationshipKind$len(vecPtr)
    }
}


public class ContentLayer: ContentLayerRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ContentLayer$_free(ptr)
        }
    }
}
public class ContentLayerRefMut: ContentLayerRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ContentLayerRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ContentLayer: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ContentLayer$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ContentLayer$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ContentLayer) {
        __swift_bridge__$Vec_ContentLayer$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ContentLayer$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ContentLayer(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContentLayerRef> {
        let pointer = __swift_bridge__$Vec_ContentLayer$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContentLayerRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ContentLayerRefMut> {
        let pointer = __swift_bridge__$Vec_ContentLayer$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ContentLayerRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ContentLayerRef> {
        UnsafePointer<ContentLayerRef>(OpaquePointer(__swift_bridge__$Vec_ContentLayer$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ContentLayer$len(vecPtr)
    }
}


public class NodeContent: NodeContentRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$NodeContent$_free(ptr)
        }
    }
}
public class NodeContentRefMut: NodeContentRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class NodeContentRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension NodeContent: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_NodeContent$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_NodeContent$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: NodeContent) {
        __swift_bridge__$Vec_NodeContent$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_NodeContent$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (NodeContent(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<NodeContentRef> {
        let pointer = __swift_bridge__$Vec_NodeContent$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return NodeContentRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<NodeContentRefMut> {
        let pointer = __swift_bridge__$Vec_NodeContent$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return NodeContentRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<NodeContentRef> {
        UnsafePointer<NodeContentRef>(OpaquePointer(__swift_bridge__$Vec_NodeContent$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_NodeContent$len(vecPtr)
    }
}


public class AnnotationKind: AnnotationKindRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$AnnotationKind$_free(ptr)
        }
    }
}
public class AnnotationKindRefMut: AnnotationKindRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class AnnotationKindRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension AnnotationKind: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_AnnotationKind$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_AnnotationKind$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: AnnotationKind) {
        __swift_bridge__$Vec_AnnotationKind$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_AnnotationKind$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (AnnotationKind(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AnnotationKindRef> {
        let pointer = __swift_bridge__$Vec_AnnotationKind$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AnnotationKindRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<AnnotationKindRefMut> {
        let pointer = __swift_bridge__$Vec_AnnotationKind$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return AnnotationKindRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<AnnotationKindRef> {
        UnsafePointer<AnnotationKindRef>(OpaquePointer(__swift_bridge__$Vec_AnnotationKind$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_AnnotationKind$len(vecPtr)
    }
}


public class ExtractionMethod: ExtractionMethodRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ExtractionMethod$_free(ptr)
        }
    }
}
public class ExtractionMethodRefMut: ExtractionMethodRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ExtractionMethodRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ExtractionMethod: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ExtractionMethod$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ExtractionMethod$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ExtractionMethod) {
        __swift_bridge__$Vec_ExtractionMethod$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ExtractionMethod$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ExtractionMethod(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionMethodRef> {
        let pointer = __swift_bridge__$Vec_ExtractionMethod$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionMethodRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ExtractionMethodRefMut> {
        let pointer = __swift_bridge__$Vec_ExtractionMethod$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ExtractionMethodRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ExtractionMethodRef> {
        UnsafePointer<ExtractionMethodRef>(OpaquePointer(__swift_bridge__$Vec_ExtractionMethod$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ExtractionMethod$len(vecPtr)
    }
}


public class ChunkType: ChunkTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ChunkType$_free(ptr)
        }
    }
}
public class ChunkTypeRefMut: ChunkTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ChunkTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ChunkType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ChunkType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ChunkType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ChunkType) {
        __swift_bridge__$Vec_ChunkType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ChunkType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ChunkType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkTypeRef> {
        let pointer = __swift_bridge__$Vec_ChunkType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ChunkTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ChunkType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ChunkTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ChunkTypeRef> {
        UnsafePointer<ChunkTypeRef>(OpaquePointer(__swift_bridge__$Vec_ChunkType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ChunkType$len(vecPtr)
    }
}


public class ImageKind: ImageKindRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageKind$_free(ptr)
        }
    }
}
public class ImageKindRefMut: ImageKindRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageKindRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageKind: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageKind$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageKind$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageKind) {
        __swift_bridge__$Vec_ImageKind$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageKind$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageKind(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageKindRef> {
        let pointer = __swift_bridge__$Vec_ImageKind$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageKindRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageKindRefMut> {
        let pointer = __swift_bridge__$Vec_ImageKind$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageKindRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageKindRef> {
        UnsafePointer<ImageKindRef>(OpaquePointer(__swift_bridge__$Vec_ImageKind$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageKind$len(vecPtr)
    }
}


public class ResultFormat: ResultFormatRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ResultFormat$_free(ptr)
        }
    }
}
public class ResultFormatRefMut: ResultFormatRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ResultFormatRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ResultFormat: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ResultFormat$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ResultFormat$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ResultFormat) {
        __swift_bridge__$Vec_ResultFormat$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ResultFormat$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ResultFormat(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ResultFormatRef> {
        let pointer = __swift_bridge__$Vec_ResultFormat$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ResultFormatRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ResultFormatRefMut> {
        let pointer = __swift_bridge__$Vec_ResultFormat$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ResultFormatRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ResultFormatRef> {
        UnsafePointer<ResultFormatRef>(OpaquePointer(__swift_bridge__$Vec_ResultFormat$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ResultFormat$len(vecPtr)
    }
}


public class ElementType: ElementTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ElementType$_free(ptr)
        }
    }
}
public class ElementTypeRefMut: ElementTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ElementTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ElementType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ElementType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ElementType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ElementType) {
        __swift_bridge__$Vec_ElementType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ElementType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ElementType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementTypeRef> {
        let pointer = __swift_bridge__$Vec_ElementType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ElementTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ElementType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ElementTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ElementTypeRef> {
        UnsafePointer<ElementTypeRef>(OpaquePointer(__swift_bridge__$Vec_ElementType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ElementType$len(vecPtr)
    }
}


public class FormatMetadata: FormatMetadataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$FormatMetadata$_free(ptr)
        }
    }
}
public class FormatMetadataRefMut: FormatMetadataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class FormatMetadataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension FormatMetadata: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_FormatMetadata$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_FormatMetadata$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: FormatMetadata) {
        __swift_bridge__$Vec_FormatMetadata$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_FormatMetadata$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (FormatMetadata(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FormatMetadataRef> {
        let pointer = __swift_bridge__$Vec_FormatMetadata$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FormatMetadataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<FormatMetadataRefMut> {
        let pointer = __swift_bridge__$Vec_FormatMetadata$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return FormatMetadataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<FormatMetadataRef> {
        UnsafePointer<FormatMetadataRef>(OpaquePointer(__swift_bridge__$Vec_FormatMetadata$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_FormatMetadata$len(vecPtr)
    }
}


public class TextDirection: TextDirectionRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TextDirection$_free(ptr)
        }
    }
}
public class TextDirectionRefMut: TextDirectionRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class TextDirectionRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TextDirection: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TextDirection$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TextDirection$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TextDirection) {
        __swift_bridge__$Vec_TextDirection$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TextDirection$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TextDirection(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextDirectionRef> {
        let pointer = __swift_bridge__$Vec_TextDirection$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextDirectionRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TextDirectionRefMut> {
        let pointer = __swift_bridge__$Vec_TextDirection$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TextDirectionRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TextDirectionRef> {
        UnsafePointer<TextDirectionRef>(OpaquePointer(__swift_bridge__$Vec_TextDirection$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TextDirection$len(vecPtr)
    }
}


public class LinkType: LinkTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LinkType$_free(ptr)
        }
    }
}
public class LinkTypeRefMut: LinkTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LinkTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LinkType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LinkType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LinkType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LinkType) {
        __swift_bridge__$Vec_LinkType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LinkType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LinkType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LinkTypeRef> {
        let pointer = __swift_bridge__$Vec_LinkType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LinkTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LinkTypeRefMut> {
        let pointer = __swift_bridge__$Vec_LinkType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LinkTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LinkTypeRef> {
        UnsafePointer<LinkTypeRef>(OpaquePointer(__swift_bridge__$Vec_LinkType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LinkType$len(vecPtr)
    }
}


public class ImageType: ImageTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ImageType$_free(ptr)
        }
    }
}
public class ImageTypeRefMut: ImageTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ImageTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ImageType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ImageType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ImageType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ImageType) {
        __swift_bridge__$Vec_ImageType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ImageType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ImageType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageTypeRef> {
        let pointer = __swift_bridge__$Vec_ImageType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ImageTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ImageType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ImageTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ImageTypeRef> {
        UnsafePointer<ImageTypeRef>(OpaquePointer(__swift_bridge__$Vec_ImageType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ImageType$len(vecPtr)
    }
}


public class StructuredDataType: StructuredDataTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$StructuredDataType$_free(ptr)
        }
    }
}
public class StructuredDataTypeRefMut: StructuredDataTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class StructuredDataTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension StructuredDataType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_StructuredDataType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_StructuredDataType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: StructuredDataType) {
        __swift_bridge__$Vec_StructuredDataType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_StructuredDataType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (StructuredDataType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataTypeRef> {
        let pointer = __swift_bridge__$Vec_StructuredDataType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<StructuredDataTypeRefMut> {
        let pointer = __swift_bridge__$Vec_StructuredDataType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return StructuredDataTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<StructuredDataTypeRef> {
        UnsafePointer<StructuredDataTypeRef>(OpaquePointer(__swift_bridge__$Vec_StructuredDataType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_StructuredDataType$len(vecPtr)
    }
}


public class OcrBoundingGeometry: OcrBoundingGeometryRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrBoundingGeometry$_free(ptr)
        }
    }
}
public class OcrBoundingGeometryRefMut: OcrBoundingGeometryRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrBoundingGeometryRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrBoundingGeometry: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrBoundingGeometry$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrBoundingGeometry$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrBoundingGeometry) {
        __swift_bridge__$Vec_OcrBoundingGeometry$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrBoundingGeometry$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrBoundingGeometry(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBoundingGeometryRef> {
        let pointer = __swift_bridge__$Vec_OcrBoundingGeometry$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBoundingGeometryRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBoundingGeometryRefMut> {
        let pointer = __swift_bridge__$Vec_OcrBoundingGeometry$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBoundingGeometryRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrBoundingGeometryRef> {
        UnsafePointer<OcrBoundingGeometryRef>(OpaquePointer(__swift_bridge__$Vec_OcrBoundingGeometry$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrBoundingGeometry$len(vecPtr)
    }
}


public class OcrElementLevel: OcrElementLevelRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrElementLevel$_free(ptr)
        }
    }
}
public class OcrElementLevelRefMut: OcrElementLevelRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrElementLevelRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrElementLevel: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrElementLevel$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrElementLevel$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrElementLevel) {
        __swift_bridge__$Vec_OcrElementLevel$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrElementLevel$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrElementLevel(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementLevelRef> {
        let pointer = __swift_bridge__$Vec_OcrElementLevel$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementLevelRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrElementLevelRefMut> {
        let pointer = __swift_bridge__$Vec_OcrElementLevel$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrElementLevelRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrElementLevelRef> {
        UnsafePointer<OcrElementLevelRef>(OpaquePointer(__swift_bridge__$Vec_OcrElementLevel$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrElementLevel$len(vecPtr)
    }
}


public class PageUnitType: PageUnitTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PageUnitType$_free(ptr)
        }
    }
}
public class PageUnitTypeRefMut: PageUnitTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PageUnitTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PageUnitType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PageUnitType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PageUnitType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PageUnitType) {
        __swift_bridge__$Vec_PageUnitType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PageUnitType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PageUnitType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageUnitTypeRef> {
        let pointer = __swift_bridge__$Vec_PageUnitType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageUnitTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PageUnitTypeRefMut> {
        let pointer = __swift_bridge__$Vec_PageUnitType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PageUnitTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PageUnitTypeRef> {
        UnsafePointer<PageUnitTypeRef>(OpaquePointer(__swift_bridge__$Vec_PageUnitType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PageUnitType$len(vecPtr)
    }
}


public class UriKind: UriKindRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$UriKind$_free(ptr)
        }
    }
}
public class UriKindRefMut: UriKindRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class UriKindRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension UriKind: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_UriKind$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_UriKind$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: UriKind) {
        __swift_bridge__$Vec_UriKind$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_UriKind$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (UriKind(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UriKindRef> {
        let pointer = __swift_bridge__$Vec_UriKind$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UriKindRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UriKindRefMut> {
        let pointer = __swift_bridge__$Vec_UriKind$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UriKindRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<UriKindRef> {
        UnsafePointer<UriKindRef>(OpaquePointer(__swift_bridge__$Vec_UriKind$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_UriKind$len(vecPtr)
    }
}


public class PoolError: PoolErrorRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PoolError$_free(ptr)
        }
    }
}
public class PoolErrorRefMut: PoolErrorRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PoolErrorRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PoolError: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PoolError$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PoolError$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PoolError) {
        __swift_bridge__$Vec_PoolError$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PoolError$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PoolError(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PoolErrorRef> {
        let pointer = __swift_bridge__$Vec_PoolError$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PoolErrorRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PoolErrorRefMut> {
        let pointer = __swift_bridge__$Vec_PoolError$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PoolErrorRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PoolErrorRef> {
        UnsafePointer<PoolErrorRef>(OpaquePointer(__swift_bridge__$Vec_PoolError$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PoolError$len(vecPtr)
    }
}


public class KeywordAlgorithm: KeywordAlgorithmRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$KeywordAlgorithm$_free(ptr)
        }
    }
}
public class KeywordAlgorithmRefMut: KeywordAlgorithmRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class KeywordAlgorithmRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension KeywordAlgorithm: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_KeywordAlgorithm$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_KeywordAlgorithm$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: KeywordAlgorithm) {
        __swift_bridge__$Vec_KeywordAlgorithm$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_KeywordAlgorithm$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (KeywordAlgorithm(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordAlgorithmRef> {
        let pointer = __swift_bridge__$Vec_KeywordAlgorithm$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordAlgorithmRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<KeywordAlgorithmRefMut> {
        let pointer = __swift_bridge__$Vec_KeywordAlgorithm$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return KeywordAlgorithmRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<KeywordAlgorithmRef> {
        UnsafePointer<KeywordAlgorithmRef>(OpaquePointer(__swift_bridge__$Vec_KeywordAlgorithm$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_KeywordAlgorithm$len(vecPtr)
    }
}


public class PSMMode: PSMModeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PSMMode$_free(ptr)
        }
    }
}
public class PSMModeRefMut: PSMModeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PSMModeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PSMMode: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PSMMode$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PSMMode$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PSMMode) {
        __swift_bridge__$Vec_PSMMode$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PSMMode$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PSMMode(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PSMModeRef> {
        let pointer = __swift_bridge__$Vec_PSMMode$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PSMModeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PSMModeRefMut> {
        let pointer = __swift_bridge__$Vec_PSMMode$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PSMModeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PSMModeRef> {
        UnsafePointer<PSMModeRef>(OpaquePointer(__swift_bridge__$Vec_PSMMode$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PSMMode$len(vecPtr)
    }
}


public class PaddleLanguage: PaddleLanguageRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PaddleLanguage$_free(ptr)
        }
    }
}
public class PaddleLanguageRefMut: PaddleLanguageRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PaddleLanguageRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PaddleLanguage: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PaddleLanguage$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PaddleLanguage$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PaddleLanguage) {
        __swift_bridge__$Vec_PaddleLanguage$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PaddleLanguage$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PaddleLanguage(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PaddleLanguageRef> {
        let pointer = __swift_bridge__$Vec_PaddleLanguage$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PaddleLanguageRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PaddleLanguageRefMut> {
        let pointer = __swift_bridge__$Vec_PaddleLanguage$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PaddleLanguageRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PaddleLanguageRef> {
        UnsafePointer<PaddleLanguageRef>(OpaquePointer(__swift_bridge__$Vec_PaddleLanguage$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PaddleLanguage$len(vecPtr)
    }
}


public class LayoutClass: LayoutClassRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$LayoutClass$_free(ptr)
        }
    }
}
public class LayoutClassRefMut: LayoutClassRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class LayoutClassRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension LayoutClass: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_LayoutClass$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_LayoutClass$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: LayoutClass) {
        __swift_bridge__$Vec_LayoutClass$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_LayoutClass$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (LayoutClass(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutClassRef> {
        let pointer = __swift_bridge__$Vec_LayoutClass$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutClassRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<LayoutClassRefMut> {
        let pointer = __swift_bridge__$Vec_LayoutClass$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return LayoutClassRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<LayoutClassRef> {
        UnsafePointer<LayoutClassRef>(OpaquePointer(__swift_bridge__$Vec_LayoutClass$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_LayoutClass$len(vecPtr)
    }
}


public class OcrBackendBox: OcrBackendBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OcrBackendBox$_free(ptr)
        }
    }
}
public class OcrBackendBoxRefMut: OcrBackendBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OcrBackendBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OcrBackendBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OcrBackendBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OcrBackendBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OcrBackendBox) {
        __swift_bridge__$Vec_OcrBackendBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OcrBackendBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OcrBackendBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBackendBoxRef> {
        let pointer = __swift_bridge__$Vec_OcrBackendBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBackendBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OcrBackendBoxRefMut> {
        let pointer = __swift_bridge__$Vec_OcrBackendBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OcrBackendBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OcrBackendBoxRef> {
        UnsafePointer<OcrBackendBoxRef>(OpaquePointer(__swift_bridge__$Vec_OcrBackendBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OcrBackendBox$len(vecPtr)
    }
}


public class PostProcessorBox: PostProcessorBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$PostProcessorBox$_free(ptr)
        }
    }
}
public class PostProcessorBoxRefMut: PostProcessorBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class PostProcessorBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension PostProcessorBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_PostProcessorBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_PostProcessorBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: PostProcessorBox) {
        __swift_bridge__$Vec_PostProcessorBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_PostProcessorBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (PostProcessorBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PostProcessorBoxRef> {
        let pointer = __swift_bridge__$Vec_PostProcessorBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PostProcessorBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<PostProcessorBoxRefMut> {
        let pointer = __swift_bridge__$Vec_PostProcessorBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return PostProcessorBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<PostProcessorBoxRef> {
        UnsafePointer<PostProcessorBoxRef>(OpaquePointer(__swift_bridge__$Vec_PostProcessorBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_PostProcessorBox$len(vecPtr)
    }
}


public class ValidatorBox: ValidatorBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$ValidatorBox$_free(ptr)
        }
    }
}
public class ValidatorBoxRefMut: ValidatorBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class ValidatorBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension ValidatorBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ValidatorBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ValidatorBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ValidatorBox) {
        __swift_bridge__$Vec_ValidatorBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ValidatorBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ValidatorBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ValidatorBoxRef> {
        let pointer = __swift_bridge__$Vec_ValidatorBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ValidatorBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ValidatorBoxRefMut> {
        let pointer = __swift_bridge__$Vec_ValidatorBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ValidatorBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ValidatorBoxRef> {
        UnsafePointer<ValidatorBoxRef>(OpaquePointer(__swift_bridge__$Vec_ValidatorBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ValidatorBox$len(vecPtr)
    }
}


public class EmbeddingBackendBox: EmbeddingBackendBoxRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$EmbeddingBackendBox$_free(ptr)
        }
    }
}
public class EmbeddingBackendBoxRefMut: EmbeddingBackendBoxRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class EmbeddingBackendBoxRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension EmbeddingBackendBox: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_EmbeddingBackendBox$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_EmbeddingBackendBox$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: EmbeddingBackendBox) {
        __swift_bridge__$Vec_EmbeddingBackendBox$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_EmbeddingBackendBox$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (EmbeddingBackendBox(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingBackendBoxRef> {
        let pointer = __swift_bridge__$Vec_EmbeddingBackendBox$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingBackendBoxRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<EmbeddingBackendBoxRefMut> {
        let pointer = __swift_bridge__$Vec_EmbeddingBackendBox$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return EmbeddingBackendBoxRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<EmbeddingBackendBoxRef> {
        UnsafePointer<EmbeddingBackendBoxRef>(OpaquePointer(__swift_bridge__$Vec_EmbeddingBackendBox$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_EmbeddingBackendBox$len(vecPtr)
    }
}


@_cdecl("__swift_bridge__$SwiftOcrBackendBox$_free")
func __swift_bridge__SwiftOcrBackendBox__free (ptr: UnsafeMutableRawPointer) {
    let _ = Unmanaged<SwiftOcrBackendBox>.fromOpaque(ptr).takeRetainedValue()
}


@_cdecl("__swift_bridge__$SwiftPostProcessorBox$_free")
func __swift_bridge__SwiftPostProcessorBox__free (ptr: UnsafeMutableRawPointer) {
    let _ = Unmanaged<SwiftPostProcessorBox>.fromOpaque(ptr).takeRetainedValue()
}


@_cdecl("__swift_bridge__$SwiftValidatorBox$_free")
func __swift_bridge__SwiftValidatorBox__free (ptr: UnsafeMutableRawPointer) {
    let _ = Unmanaged<SwiftValidatorBox>.fromOpaque(ptr).takeRetainedValue()
}


@_cdecl("__swift_bridge__$SwiftEmbeddingBackendBox$_free")
func __swift_bridge__SwiftEmbeddingBackendBox__free (ptr: UnsafeMutableRawPointer) {
    let _ = Unmanaged<SwiftEmbeddingBackendBox>.fromOpaque(ptr).takeRetainedValue()
}