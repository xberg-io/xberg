```ruby title="Ruby"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Access PDF metadata
if result.metadata['pdf']
  pdf_meta = result.metadata['pdf']
  puts "Pages: #{pdf_meta['page_count']}"
  puts "Author: #{pdf_meta['author']}"
  puts "Title: #{pdf_meta['title']}"
end

# Access HTML metadata
html_result = Kreuzberg.extract_file_sync('page.html')
if html_result.metadata['html']
  html_meta = html_result.metadata['html']
  puts "Title: #{html_meta['title']}"
  puts "Description: #{html_meta['description']}"

  # Access keywords as array
  puts "Keywords: #{html_meta['keywords']}"

  # Access canonical URL (renamed from canonical)
  puts "Canonical URL: #{html_meta['canonical_url']}" if html_meta['canonical_url']

  # Access Open Graph fields from map
  open_graph = html_meta['open_graph'] || {}
  puts "Open Graph Image: #{open_graph['image']}" if open_graph['image']
  puts "Open Graph Title: #{open_graph['title']}" if open_graph['title']
  puts "Open Graph Type: #{open_graph['type']}" if open_graph['type']

  # Access Twitter Card fields from map
  twitter_card = html_meta['twitter_card'] || {}
  puts "Twitter Card Type: #{twitter_card['card']}" if twitter_card['card']
  puts "Twitter Creator: #{twitter_card['creator']}" if twitter_card['creator']

  # Access new fields
  puts "Language: #{html_meta['language']}" if html_meta['language']
  puts "Text Direction: #{html_meta['text_direction']}" if html_meta['text_direction']

  # Access headers
  if html_meta['headers']
    puts "Headers: #{html_meta['headers'].join(', ')}"
  end

  # Access links
  if html_meta['links']
    html_meta['links'].each do |link|
      puts "Link: #{link['href']} (#{link['text']})"
    end
  end

  # Access images
  if html_meta['images']
    html_meta['images'].each do |image|
      puts "Image: #{image['src']}"
    end
  end

  # Access structured data
  if html_meta['structured_data']
    puts "Structured data items: #{html_meta['structured_data'].length}"
  end
end
```
