```bash title="Bash"
# Run server on port 8000
docker run -d \
  -e XBERG_API_TOKEN="$XBERG_API_TOKEN" \
  -p 8000:8000 \
  ghcr.io/xberg-io/xberg:latest \
  serve -H 0.0.0.0 -p 8000

# With environment variables
docker run -d \
  -e XBERG_API_TOKEN="$XBERG_API_TOKEN" \
  -e XBERG_CORS_ORIGINS="https://myapp.com" \
  -e XBERG_MAX_MULTIPART_FIELD_BYTES=209715200 \
  -p 8000:8000 \
  ghcr.io/xberg-io/xberg:latest \
  serve -H 0.0.0.0 -p 8000

```
