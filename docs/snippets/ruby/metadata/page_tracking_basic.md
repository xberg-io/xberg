require 'xberg'

config = Xberg::ExtractionConfig.new(
  pages: Xberg::PageConfig.new(
    extract_pages: true
  )
)

input = Xberg::ExtractInput.new(uri: 'document.pdf')
result = Xberg.extract(input, config)

result.results.first.pages&.each do |page|
  puts "Page #{page.page_number}:"
  puts " Content: #{page.content.length} chars"
  puts " Tables: #{page.tables.length}"
  puts " Images: #{page.images.length}"
end
