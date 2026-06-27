```ruby title="Ruby"
require 'xberg'

class StatefulPlugin
  def initialize
    @lock = Mutex.new
    @count = 0
  end

  def call(result)
    @lock.synchronize { @count += 1 }
    result
  end

  def count
    @lock.synchronize { @count }
  end
end

plugin = StatefulPlugin.new
Xberg.register_post_processor('counter', plugin)

config = Xberg::ExtractionConfig.new(
  postprocessor: { enabled: true }
)

Xberg.extract_sync('document.pdf', config: config)
puts "Processed: #{plugin.count}"
```
