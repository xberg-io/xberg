```gleam title="Gleam"
import gleam/bit_array
import gleam/http
import gleam/http/request
import gleam/httpc
import gleam/int
import gleam/io
import simplifile

// Calls the Kreuzberg HTTP API server's `/extract` endpoint.
// Sends the raw file bytes with the appropriate `Content-Type` header.
pub fn main() {
  let assert Ok(bytes) = simplifile.read_bits("document.pdf")
  let assert Ok(req) = request.to("http://localhost:8000/extract")

  let req =
    req
    |> request.set_method(http.Post)
    |> request.set_header("content-type", "application/pdf")
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
