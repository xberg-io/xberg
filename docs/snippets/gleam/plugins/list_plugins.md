```gleam title="Gleam"
import gleam/io
import gleam/string
import kreuzberg

pub fn main() {
  let assert Ok(extractors) = kreuzberg.list_document_extractors()
  let assert Ok(processors) = kreuzberg.list_post_processors()
  let assert Ok(validators) = kreuzberg.list_validators()
  let assert Ok(backends) = kreuzberg.list_ocr_backends()

  io.println("Document extractors: " <> string.join(extractors, ", "))
  io.println("Post-processors: " <> string.join(processors, ", "))
  io.println("Validators: " <> string.join(validators, ", "))
  io.println("OCR backends: " <> string.join(backends, ", "))
}
```
