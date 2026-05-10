```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  pdf_options: Kreuzberg::PdfConfig.new(
    extract_images: true,
    extract_metadata: true,
    passwords: ['password1', 'password2'],
    hierarchy: Kreuzberg::HierarchyConfig.new(
      enabled: true,
      k_clusters: 6,
      include_bbox: true
    )
  )
)
```
