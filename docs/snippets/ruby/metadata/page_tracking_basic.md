Require 'Kreuzberg'

Config = Kreuzberg::ExtractionConfig.new(
pages: Kreuzberg::PageConfig.new(
extract_pages: true
)
)

Result = Kreuzberg.extract_file_sync("document.pdf", config: config)

Result.pages&.each do |page|
puts "Page #{page.page_number}:"
puts " Content: #{page.content.length} chars"
puts " Tables: #{page.tables.length}"
puts " Images: #{page.images.length}"
end
