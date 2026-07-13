# Plugin trait bridge for `Renderer` — a Crystal object registered into the Rust
# `Renderer` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct RendererVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    render_result : (Void*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_renderer = xberg_register_renderer(name : LibC::Char*, vtable : RendererVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_renderer = xberg_unregister_renderer(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@renderer_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `Renderer`.
  abstract class Renderer
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
    # Binding-safe rendering entry point for foreign-language plugin bridges.
    def render_result(result : ExtractedDocument) : String
      raise "not implemented: render_result"
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

  # Register a Crystal `Renderer` implementation into the Rust registry.
  def self.register_renderer(name : String, impl : Renderer) : Bool
    ud = Box.box(impl)
    @@renderer_plugins[name] = ud
    vtable = LibXberg::RendererVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(Renderer).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(Renderer).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(Renderer).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(Renderer).unbox(user_data).shutdown
      0
    end
    vtable.render_result = ->(user_data : Void*, result : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __result = ExtractedDocument.from_json(String.new(result))
      begin
        out_result.value = Xberg.__alef_dup_cstr(Box(Renderer).unbox(user_data).render_result(__result))
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_renderer(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `Renderer` implementation.
  def self.unregister_renderer(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_renderer(name, pointerof(out_error)) == 0
    @@renderer_plugins.delete(name)
    ok
  end
end
