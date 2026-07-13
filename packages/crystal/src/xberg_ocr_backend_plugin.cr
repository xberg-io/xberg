# Plugin trait bridge for `OcrBackend` — a Crystal object registered into the Rust
# `OcrBackend` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct OcrBackendVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    process_image : (Void*, LibC::Char*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    process_image_file : (Void*, LibC::Char*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    supports_language : (Void*, LibC::Char*) -> Int32
    backend_type : (Void*, LibC::Char**, LibC::Char**) -> Int32
    supported_languages : (Void*, LibC::Char**, LibC::Char**) -> Int32
    supports_table_detection : (Void*) -> Int32
    supports_document_processing : (Void*) -> Int32
    emits_structured_markdown : (Void*) -> Int32
    process_document : (Void*, LibC::Char*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_ocr_backend = xberg_register_ocr_backend(name : LibC::Char*, vtable : OcrBackendVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_ocr_backend = xberg_unregister_ocr_backend(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@ocr_backend_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `OcrBackend`.
  abstract class OcrBackend
    def name : String
      ""
    end
    def version : String
      "0.0.0"
    end
    def initialize_plugin : Nil
    end
    def shutdown : Nil
    end
    # Process an image and extract text via OCR.
    def process_image(image_bytes : Bytes, config : OcrConfig) : ExtractedDocument
      raise "not implemented: process_image"
    end
    # Process a file and extract text via OCR.
    def process_image_file(path : String, config : OcrConfig) : ExtractedDocument
      raise "not implemented: process_image_file"
    end
    # Check if this backend supports a given language code.
    def supports_language(lang : String) : Bool
      raise "not implemented: supports_language"
    end
    # Get the backend type identifier.
    def backend_type : OcrBackendType
      raise "not implemented: backend_type"
    end
    # Optional: Get a list of all supported languages.
    def supported_languages : Array(String)
      raise "not implemented: supported_languages"
    end
    # Optional: Check if the backend supports table detection.
    def supports_table_detection : Bool
      raise "not implemented: supports_table_detection"
    end
    # Check if the backend supports direct document-level processing (e.g. for PDFs).
    def supports_document_processing : Bool
      raise "not implemented: supports_document_processing"
    end
    # Declare that this backend emits structured markdown directly (tables, headings, lists)
    def emits_structured_markdown : Bool
      raise "not implemented: emits_structured_markdown"
    end
    # Process a document file directly via OCR.
    def process_document(path : String, config : OcrConfig) : ExtractedDocument
      raise "not implemented: process_document"
    end
  end

  # Copy a Crystal String to a malloc'd NUL-terminated C string (Rust frees it via free_string).
  def self.__alef_dup_cstr(s : String) : LibC::Char*
    bytes = s.to_slice
    buf = LibC.malloc(bytes.size + 1).as(UInt8*)
    buf.copy_from(bytes.to_unsafe, bytes.size)
    buf[bytes.size] = 0_u8
    buf.as(LibC::Char*)
  end

  # Register a Crystal `OcrBackend` implementation into the Rust registry.
  def self.register_ocr_backend(name : String, impl : OcrBackend) : Bool
    ud = Box.box(impl)
    @@ocr_backend_plugins[name] = ud
    vtable = LibXberg::OcrBackendVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(OcrBackend).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(OcrBackend).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(OcrBackend).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(OcrBackend).unbox(user_data).shutdown
      0
    end
    vtable.process_image = ->(user_data : Void*, image_bytes : LibC::Char*, config : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __image_bytes = (begin; __arr = Array(UInt8).from_json(String.new(image_bytes)); Bytes.new(__arr.to_unsafe, __arr.size); end)
      __config = OcrConfig.from_json(String.new(config))
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(OcrBackend).unbox(user_data).process_image(__image_bytes, __config)).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.process_image_file = ->(user_data : Void*, path : LibC::Char*, config : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __path = String.new(path)
      __config = OcrConfig.from_json(String.new(config))
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(OcrBackend).unbox(user_data).process_image_file(__path, __config)).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.supports_language = ->(user_data : Void*, lang : LibC::Char*) do
      __lang = String.new(lang)
      (Box(OcrBackend).unbox(user_data).supports_language(__lang)) ? 1 : 0
    end
    vtable.backend_type = ->(user_data : Void*, out_result : LibC::Char**, out_error : LibC::Char**) do
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(OcrBackend).unbox(user_data).backend_type()).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.supported_languages = ->(user_data : Void*, out_result : LibC::Char**, out_error : LibC::Char**) do
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(OcrBackend).unbox(user_data).supported_languages()).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.supports_table_detection = ->(user_data : Void*) do
      (Box(OcrBackend).unbox(user_data).supports_table_detection()) ? 1 : 0
    end
    vtable.supports_document_processing = ->(user_data : Void*) do
      (Box(OcrBackend).unbox(user_data).supports_document_processing()) ? 1 : 0
    end
    vtable.emits_structured_markdown = ->(user_data : Void*) do
      (Box(OcrBackend).unbox(user_data).emits_structured_markdown()) ? 1 : 0
    end
    vtable.process_document = ->(user_data : Void*, path : LibC::Char*, config : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __path = String.new(path)
      __config = OcrConfig.from_json(String.new(config))
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(OcrBackend).unbox(user_data).process_document(__path, __config)).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_ocr_backend(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `OcrBackend` implementation.
  def self.unregister_ocr_backend(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_ocr_backend(name, pointerof(out_error)) == 0
    @@ocr_backend_plugins.delete(name)
    ok
  end
end
