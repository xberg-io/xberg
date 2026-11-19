```bash title="Bash"
# Run server on port 8000
docker run -d \n  -e XBERG_API_TOKEN="$XBERG_API_TOKEN" \n  -p 8000:8000 \n  ghcr.io/xberg-io/xberg:latest \n  serve -H 0.0.0.0 -p 8000

# With environment variables
docker run -d \n  -e XBERG_API_TOKEN="$XBERG_API_TOKEN" \n  -e XBERG_CORS_ORIGINS="https://myapp.com" \n  -e XBERG_MAX_MULTIPART_FIELD_BYTES=209715200 \n  -p 8000:8000 \n  ghcr.io/xberg-io/xberg:latest \n  serve -H 0.0.0.0 -p 8000

```
