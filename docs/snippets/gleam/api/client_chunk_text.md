```gleam title="Gleam"
import gleam/bit_array
import gleam/http
import gleam/http/request
import gleam/httpc
import gleam/int
import gleam/io
import simplifile

// Calls the Kreuzberg HTTP API server's `/extract` endpoint with chunking
// configured via the `X-Kreuzberg-Config` header (JSON-encoded inline config).
pub fn main() {
  let assert Ok(bytes) = simplifile.read_bits("document.pdf")
  let assert Ok(req) = request.to("http://localhost:8000/extract")

  let chunking_config =
    "{\"chunking\":{\"max_characters\":800,\"overlap\":100}}"

  let req =
    req
    |> request.set_method(http.Post)
    |> request.set_header("content-type", "application/pdf")
    |> request.set_header("x-kreuzberg-config", chunking_config)
    |> request.set_body(bytes)

  case httpc.send_bits(req) {
    Ok(response) -> {
      io.println("status: " <> int.to_string(response.status))
      case bit_array.to_string(response.body) {
        Ok(body) -> io.println(body)
        Error(_) -> io.println("(non-utf8 response body)")
      }
    }
    Error(_) -> io.println_error("HTTP request failed")
  }
}
```
