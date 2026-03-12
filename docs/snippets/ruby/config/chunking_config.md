```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_characters: 1000,
    overlap: 200
  )
)
```

```ruby title="Ruby - Markdown with Heading Context"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    chunker_type: "markdown",
    max_characters: 500,
    overlap: 50,
    sizing_type: "tokenizer",
    sizing_model: "Xenova/gpt-4o"
  )
)

result = Kreuzberg.extract_file("document.md", config)

result.chunks.each do |chunk|
  if chunk.metadata.heading_context
    puts "Headings:"
    chunk.metadata.heading_context.headings.each do |heading|
      puts "  #{' ' * (heading.level - 1) * 2}Level #{heading.level}: #{heading.text}"
    end
  end
end
```
