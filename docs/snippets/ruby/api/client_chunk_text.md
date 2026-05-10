```ruby title="Ruby"
require 'net/http'
require 'json'

uri = URI('http://localhost:8000/chunk')
http = Net::HTTP.new(uri.host, uri.port)

request = Net::HTTP::Post.new(uri)
request['Content-Type'] = 'application/json'

payload = {
  text: 'Your long text content here...',
  chunker_type: 'text',
  config: {
    max_characters: 1000,
    overlap: 50,
    trim: true
  }
}

request.body = JSON.generate(payload)

response = http.request(request)

if response.is_a?(Net::HTTPSuccess)
  result = JSON.parse(response.body)
  puts "Created #{result['chunk_count']} chunks"
  
  result['chunks'].each do |chunk|
    preview = chunk['content'][0..49]
    puts "Chunk #{chunk['chunk_index']}: #{preview}..."
  end
else
  puts "Error: #{response.code} #{response.message}"
end
```
