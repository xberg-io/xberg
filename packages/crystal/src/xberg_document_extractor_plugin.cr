# Plugin trait bridge for `DocumentExtractor` — a Crystal object registered into the Rust
# `DocumentExtractor` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct DocumentExtractorVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    extract : (Void*, LibC::Char*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    supported_mime_types : (Void*, LibC::Char**, LibC::Char**) -> Int32
    priority : (Void*) -> Int32
    can_handle : (Void*, LibC::Char*, LibC::Char*) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_document_extractor = xberg_register_document_extractor(name : LibC::Char*, vtable : DocumentExtractorVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_document_extractor = xberg_unregister_document_extractor(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@document_extractor_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `DocumentExtractor`.
  abstract class DocumentExtractor
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
    # Binding-safe extraction entry point for foreign-language plugin bridges.
    def extract(input : ExtractInput, config : ExtractionConfig) : ExtractedDocument
      raise "not implemented: extract"
    end
    # Get the list of MIME types supported by this extractor.
    def supported_mime_types : Array(String)
      raise "not implemented: supported_mime_types"
    end
    # Get the priority of this extractor.
    def priority : Int32
      raise "not implemented: priority"
    end
    # Optional: Check if this extractor can handle a specific file.
    def can_handle(path : String, mime_type : String) : Bool
      raise "not implemented: can_handle"
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

  # Register a Crystal `DocumentExtractor` implementation into the Rust registry.
  def self.register_document_extractor(name : String, impl : DocumentExtractor) : Bool
    ud = Box.box(impl)
    @@document_extractor_plugins[name] = ud
    vtable = LibXberg::DocumentExtractorVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(DocumentExtractor).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(DocumentExtractor).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(DocumentExtractor).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(DocumentExtractor).unbox(user_data).shutdown
      0
    end
    vtable.extract = ->(user_data : Void*, input : LibC::Char*, config : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __input = ExtractInput.from_json(String.new(input))
      __config = ExtractionConfig.from_json(String.new(config))
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(DocumentExtractor).unbox(user_data).extract(__input, __config)).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.supported_mime_types = ->(user_data : Void*, out_result : LibC::Char**, out_error : LibC::Char**) do
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(DocumentExtractor).unbox(user_data).supported_mime_types()).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.priority = ->(user_data : Void*) do
      Box(DocumentExtractor).unbox(user_data).priority()
    end
    vtable.can_handle = ->(user_data : Void*, path : LibC::Char*, mime_type : LibC::Char*) do
      __path = String.new(path)
      __mime_type = String.new(mime_type)
      (Box(DocumentExtractor).unbox(user_data).can_handle(__path, __mime_type)) ? 1 : 0
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_document_extractor(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `DocumentExtractor` implementation.
  def self.unregister_document_extractor(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_document_extractor(name, pointerof(out_error)) == 0
    @@document_extractor_plugins.delete(name)
    ok
  end
end
