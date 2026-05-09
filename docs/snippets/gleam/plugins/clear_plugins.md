```gleam title="Gleam"
import gleam/io
import kreuzberg

pub fn main() {
  let assert Ok(Nil) = kreuzberg.clear_post_processors()
  let assert Ok(Nil) = kreuzberg.clear_validators()
  let assert Ok(Nil) = kreuzberg.clear_ocr_backends()
  io.println("registries cleared")
}
```
