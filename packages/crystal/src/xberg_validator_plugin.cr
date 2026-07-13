# Plugin trait bridge for `Validator` — a Crystal object registered into the Rust
# `Validator` registry, implementing the trait across the C-ABI vtable.
require "json"

lib LibXberg
  struct ValidatorVTable
    name_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    version_fn : (Void*, LibC::Char**, LibC::Char**) -> Int32
    initialize_fn : (Void*, LibC::Char**) -> Int32
    shutdown_fn : (Void*, LibC::Char**) -> Int32
    validate : (Void*, LibC::Char*, LibC::Char*, LibC::Char**) -> Int32
    should_validate : (Void*, LibC::Char*, LibC::Char*) -> Int32
    priority : (Void*) -> Int32
    free_string : (LibC::Char*) -> Void
    free_user_data : (Void*) -> Void
  end

  fun register_validator = xberg_register_validator(name : LibC::Char*, vtable : ValidatorVTable*, user_data : Void*, out_error : LibC::Char**) : Int32
  fun unregister_validator = xberg_unregister_validator(name : LibC::Char*, out_error : LibC::Char**) : Int32
end

module Xberg
  @@validator_plugins = {} of String => Void*

  # Subclass and override the trait methods to implement `Validator`.
  abstract class Validator
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
    # Validate an extraction result.
    def validate(result : ExtractedDocument, config : ExtractionConfig) : Nil
      raise "not implemented: validate"
    end
    # Optional: Check if this validator should run for a given result.
    def should_validate(result : ExtractedDocument, config : ExtractionConfig) : Bool
      raise "not implemented: should_validate"
    end
    # Optional: Get the validation priority.
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

  # Register a Crystal `Validator` implementation into the Rust registry.
  def self.register_validator(name : String, impl : Validator) : Bool
    ud = Box.box(impl)
    @@validator_plugins[name] = ud
    vtable = LibXberg::ValidatorVTable.new
    vtable.name_fn = ->(user_data : Void*, out_name : LibC::Char**, out_error : LibC::Char**) do
      out_name.value = Xberg.__alef_dup_cstr(Box(Validator).unbox(user_data).name)
      0
    end
    vtable.version_fn = ->(user_data : Void*, out_version : LibC::Char**, out_error : LibC::Char**) do
      out_version.value = Xberg.__alef_dup_cstr(Box(Validator).unbox(user_data).version)
      0
    end
    vtable.initialize_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(Validator).unbox(user_data).initialize_plugin
      0
    end
    vtable.shutdown_fn = ->(user_data : Void*, out_error : LibC::Char**) do
      Box(Validator).unbox(user_data).shutdown
      0
    end
    vtable.validate = ->(user_data : Void*, result : LibC::Char*, config : LibC::Char*, out_error : LibC::Char**) do
      __result = ExtractedDocument.from_json(String.new(result))
      __config = ExtractionConfig.from_json(String.new(config))
      begin
        Box(Validator).unbox(user_data).validate(__result, __config)
        0
      rescue __ex
        out_error.value = Xberg.__alef_dup_cstr(__ex.message || "error")
        1
      end
    end
    vtable.should_validate = ->(user_data : Void*, result : LibC::Char*, config : LibC::Char*) do
      __result = ExtractedDocument.from_json(String.new(result))
      __config = ExtractionConfig.from_json(String.new(config))
      (Box(Validator).unbox(user_data).should_validate(__result, __config)) ? 1 : 0
    end
    vtable.priority = ->(user_data : Void*) do
      Box(Validator).unbox(user_data).priority()
    end
    vtable.free_string = ->(p : LibC::Char*) { LibC.free(p.as(Void*)) }
    out_error = Pointer(LibC::Char).null
    LibXberg.register_validator(name, pointerof(vtable), ud, pointerof(out_error)) == 0
  end

  # Unregister a previously registered `Validator` implementation.
  def self.unregister_validator(name : String) : Bool
    out_error = Pointer(LibC::Char).null
    ok = LibXberg.unregister_validator(name, pointerof(out_error)) == 0
    @@validator_plugins.delete(name)
    ok
  end
end
