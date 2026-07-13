# Plugin trait bridge for `RerankerBackend` — a Crystal object registered into the Rust
# `RerankerBackend` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct RerankerBackendVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    rerank : (Void*, LibC::Char*, LibC::Char*, LibC::Char**, LibC::Char**) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_reranker_backend = xberg_register_reranker_backend(name : LibC::Char*, vtable : RerankerBackendVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_reranker_backend = xberg_unregister_reranker_backend(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@reranker_backend_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `RerankerBackend`.
  abstract class RerankerBackend
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
    # Score a list of documents against a query.
    def rerank(query : String, documents : Array(String)) : Array(Float32)
      raise "not implemented: rerank"
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

  # Register a Crystal `RerankerBackend` implementation into the Rust registry.
  def self.register_reranker_backend(name : String, impl : RerankerBackend) : Bool
    ud = Box.box(impl)
    @@reranker_backend_plugins[name] = ud
    vtable = LibXberg::RerankerBackendVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(RerankerBackend).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(RerankerBackend).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(RerankerBackend).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(RerankerBackend).unbox(user_data).shutdown
      0
    end
    vtable.rerank = ->(user_data : Void*, query : LibC::Char*, documents : LibC::Char*, out_result : LibC::Char**, out_error : LibC::Char**) do
      __query = String.new(query)
      __documents = Array(String).from_json(String.new(documents))
      begin
        out_result.value = Xberg.__alef_dup_cstr((Box(RerankerBackend).unbox(user_data).rerank(__query, __documents)).to_json)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_reranker_backend(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `RerankerBackend` implementation.
  def self.unregister_reranker_backend(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_reranker_backend(name, pointerof(out_error)) == 0
    @@reranker_backend_plugins.delete(name)
    ok
  end
end
