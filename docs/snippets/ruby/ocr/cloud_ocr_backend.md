```ruby title="Ruby"
require 'xberg'
require 'net/http'

class CloudOcrBackend
  def name
    'cloud-ocr'
  end

  def supported_languages
    %w[eng fra deu]
  end

  def process_image(image_data, language)
    uri = URI('https://api.example.com/ocr')
    req = Net::HTTP::Post.new(uri)
    req['Authorization'] = "Bearer #{ENV['OCR_API_KEY']}"
    req.body = image_data
    res = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) { |h| h.request(req) }
    raise StandardError, res.message unless res.is_a?(Net::HTTPSuccess)
    { content: JSON.parse(res.body)['text'] }
  rescue StandardError => e
    raise StandardError, e.message
  end
end

Xberg.register_ocr_backend(CloudOcrBackend.new)
config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(backend: 'cloud-ocr')
)
Xberg.extract_sync('doc.pdf', config: config)
```
