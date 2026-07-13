# Xberg Crystal — usage examples
#
# Build:  cd packages/crystal && make example
# Run:    bin/example

require "../src/xberg"

# ── Plugin: register and unregister a custom OCR backend ──────────

class MyOcrBackend < Xberg::OcrBackend
  def process_image(image_bytes : Bytes, config : Xberg::OcrConfig) : Xberg::ExtractedDocument
    Xberg::ExtractedDocument.from_json(%({
      "content": "OCR extracted text from custom backend",
      "mime_type": "text/plain",
      "language": "eng"
    }))
  end

  def process_image_file(path : String, config : Xberg::OcrConfig) : Xberg::ExtractedDocument
    process_image(File.read(path).to_slice, config)
  end

  def supports_language(lang : String) : Bool
    ["eng", "deu", "fra"].includes?(lang)
  end

  def backend_type : Xberg::OcrBackendType
    Xberg::OcrBackendType::Custom
  end
end

puts "Registering OCR backend..."
Xberg.register_ocr_backend("my-ocr", MyOcrBackend.new)
puts "  ✓ Registered"

puts "Listing backends..."
backends = Xberg.list_ocr_backends
puts "  Backends: #{backends}"

puts "Unregistering..."
Xberg.unregister_ocr_backend("my-ocr")
puts "  ✓ Unregistered"

puts "\nCrystal xberg bindings working!"
