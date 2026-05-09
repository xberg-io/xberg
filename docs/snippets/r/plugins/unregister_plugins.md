<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

# Remove plugins by their registered name.
unregister_post_processor("metadata_enrichment")
unregister_validator("min_length")
unregister_ocr_backend("custom_ocr_backend")
unregister_document_extractor("custom_format")
```
