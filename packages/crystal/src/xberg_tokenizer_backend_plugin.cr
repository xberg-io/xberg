# Plugin trait bridge for `TokenizerBackend` — a Crystal object registered into the Rust
# `TokenizerBackend` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct TokenizerBackendVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    count_tokens : (Void*, LibC::Char*) -> LibC::SizeT
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_tokenizer_backend = xberg_register_tokenizer_backend(name : LibC::Char*, vtable : TokenizerBackendVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_tokenizer_backend = xberg_unregister_tokenizer_backend(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@tokenizer_backend_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `TokenizerBackend`.
  abstract class TokenizerBackend
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
    # Count the tokens in `text` according to this backend's tokenizer.
    def count_tokens(text : String) : UInt64
      raise "not implemented: count_tokens"
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

  # Register a Crystal `TokenizerBackend` implementation into the Rust registry.
  def self.register_tokenizer_backend(name : String, impl : TokenizerBackend) : Bool
    ud = Box.box(impl)
    @@tokenizer_backend_plugins[name] = ud
    vtable = LibXberg::TokenizerBackendVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(TokenizerBackend).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(TokenizerBackend).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(TokenizerBackend).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(TokenizerBackend).unbox(user_data).shutdown
      0
    end
    vtable.count_tokens = ->(user_data : Void*, text : LibC::Char*) do
      __text = String.new(text)
      Box(TokenizerBackend).unbox(user_data).count_tokens(__text)
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_tokenizer_backend(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `TokenizerBackend` implementation.
  def self.unregister_tokenizer_backend(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_tokenizer_backend(name, pointerof(out_error)) == 0
    @@tokenizer_backend_plugins.delete(name)
    ok
  end
end
