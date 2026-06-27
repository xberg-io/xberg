Require 'Xberg'

Config = Xberg::ExtractionConfig.new(
pages: Xberg::PageConfig.new(
extract_pages: true
)
)

Result = Xberg.extract_sync("document.pdf", config: config)

Result.pages&.each do |page|
puts "Page #{page.page_number}:"
puts " Content: #{page.content.length} chars"
puts " Tables: #{page.tables.length}"
puts " Images: #{page.images.length}"
end
