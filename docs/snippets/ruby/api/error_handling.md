```ruby title="Ruby"
require 'xberg'

begin
  result = Xberg.extract_sync('missing.pdf')
  puts result.content
rescue RuntimeError => e
  # All extraction errors are raised as RuntimeError
  # Check error message for specific error details
  case e.message
  when /validation/i
    puts "Validation error: #{e.message}"
  when /io|not found/i
    puts "IO error: #{e.message}"
    raise
  else
    puts "Extraction failed: #{e.message}"
    raise
  end
end
```
