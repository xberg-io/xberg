```ruby title="Ruby"
require 'net/http'
require 'json'

uri = URI('http://localhost:8000/extract')
http = Net::HTTP.new(uri.host, uri.port)

request = Net::HTTP::Post.new(uri)

File.open('document.pdf', 'rb') do |file|
  body = file.read
  request['Content-Type'] = 'application/octet-stream'
  request.body = body
  
  response = http.request(request)
  
  if response.is_a?(Net::HTTPSuccess)
    data = JSON.parse(response.body)
    puts JSON.pretty_generate(data)
  else
    puts "Error: #{response.code} #{response.message}"
  end
end
```
