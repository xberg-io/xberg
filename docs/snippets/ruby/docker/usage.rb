```ruby title="usage.rb"
require 'open3'
require 'net/http'
require 'json'

class DockerXbergClient
  def initialize(container_name = 'xberg-api', api_port = 8000)
    @container_name = container_name
    @api_port = api_port
    @api_url = "http://localhost:#{api_port}/api/extract"
  end

  def start_container(image = 'xberg:latest')
    puts 'Starting Xberg Docker container...'
    cmd = "docker run -d --name #{@container_name} -p #{@api_port}:8000 #{image}"
    stdout, stderr, status = Open3.capture3(cmd)

    raise "Failed to start container: #{stderr}" unless status.success?

    puts "Container started on http://localhost:#{@api_port}"
  end

  def extract(file_path)
    file_content = File.read(file_path, mode: 'rb')
    boundary = "----WebKitFormBoundary#{SecureRandom.hex(16)}"

    body = "--#{boundary}\r\n"
    body += "Content-Disposition: form-data; name=\"file\"; filename=\"#{File.basename(file_path)}\"\r\n"
    body += "Content-Type: application/octet-stream\r\n\r\n"
    body += file_content
    body += "\r\n--#{boundary}--\r\n"

    uri = URI(@api_url)
    http = Net::HTTP.new(uri.host, uri.port)
    request = Net::HTTP::Post.new(uri.path)
    request['Content-Type'] = "multipart/form-data; boundary=#{boundary}"
    request.body = body

    response = http.request(request)
    result = JSON.parse(response.body)
    result['content']
  end

  def stop_container
    puts 'Stopping Xberg Docker container...'
    system("docker stop #{@container_name}")
    system("docker rm #{@container_name}")
    puts 'Container stopped and removed'
  end
end

docker_client = DockerXbergClient.new

begin
  docker_client.start_container
  sleep(2)

  content = docker_client.extract('document.pdf')
  puts "Extracted content:\n#{content}"
ensure
  docker_client.stop_container
end
```
