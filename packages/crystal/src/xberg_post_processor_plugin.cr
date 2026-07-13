# Plugin trait bridge for `PostProcessor` — a Crystal object registered into the Rust
# `PostProcessor` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct PostProcessorVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    process : (Void*, LibC::Char*, LibC::Char*, LibC::Char**) -> Int32
    processing_stage : (Void*, LibC::Char**, LibC::Char**) -> Int32
    should_process : (Void*, LibC::Char*, LibC::Char*) -> Int32
    estimated_duration_ms : (Void*, LibC::Char*) -> UInt64
    priority : (Void*) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_post_processor = xberg_register_post_processor(name : LibC::Char*, vtable : PostProcessorVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_post_processor = xberg_unregister_post_processor(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@post_processor_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `PostProcessor`.
  abstract class PostProcessor
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
    # Process an extraction result.
    def process(result : ExtractedDocument, config : ExtractionConfig) : Nil
      raise "not implemented: process"
    end
    # Get the processing stage for this post-processor.
    def processing_stage : ProcessingStage
      raise "not implemented: processing_stage"
    end
    # Optional: Check if this processor should run for a given result.
    def should_process(result : ExtractedDocument, config : ExtractionConfig) : Bool
      raise "not implemented: should_process"
    end
    # Optional: Estimate processing time in milliseconds.
    def estimated_duration_ms(result : ExtractedDocument) : UInt64
      raise "not implemented: estimated_duration_ms"
    end
    # Execution priority within the processing stage.
    def priority : Int32
      raise "not implemented: priority"
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

  # Register a Crystal `PostProcessor` implementation into the Rust registry.
  def self.register_post_processor(name : String, impl : PostProcessor) : Bool
    ud = Box.box(impl)
    @@post_processor_plugins[name] = ud
    vtable = LibXberg::PostProcessorVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(PostProcessor).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(PostProcessor).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(PostProcessor).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(PostProcessor).unbox(user_data).shutdown
      0
    end
    vtable.process = ->(user_data : Void*, result : LibC::Char*, config : LibC::Char*, out_error : LibC::Char**) do
      __result = ExtractedDocument.from_json(String.new(result))
      __config = ExtractionConfig.from_json(String.new(config))
      begin
        Box(PostProcessor).unbox(user_data).process(__result, __config)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.processing_stage = ->(user_data : Void*, out_result : LibC::Char**, out_error : LibC::Char**) do
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(PostProcessor).unbox(user_data).processing_stage()).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.should_process = ->(user_data : Void*, result : LibC::Char*, config : LibC::Char*) do
      __result = ExtractedDocument.from_json(String.new(result))
      __config = ExtractionConfig.from_json(String.new(config))
      (Box(PostProcessor).unbox(user_data).should_process(__result, __config)) ? 1 : 0
    end
    vtable.estimated_duration_ms = ->(user_data : Void*, result : LibC::Char*) do
      __result = ExtractedDocument.from_json(String.new(result))
      Box(PostProcessor).unbox(user_data).estimated_duration_ms(__result)
    end
    vtable.priority = ->(user_data : Void*) do
      Box(PostProcessor).unbox(user_data).priority()
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_post_processor(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `PostProcessor` implementation.
  def self.unregister_post_processor(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_post_processor(name, pointerof(out_error)) == 0
    @@post_processor_plugins.delete(name)
    ok
  end
end
