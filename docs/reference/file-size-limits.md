# File Size Limits

Kreuzberg enforces size limits on file uploads and API requests to manage server resources effectively. This page documents the default limits, how to configure them, and recommendations for optimal performance.

## Overview

File size limits protect your server from resource exhaustion and unexpected memory spikes. The Kreuzberg API implements two complementary limit types:

| Limit Type | Purpose | Default |
|------------|---------|---------|
| **Request Body Limit** | Total size of all files in a single request | 100 MB |
| **Multipart Field Limit** | Maximum size of an individual file | 100 MB |

Both limits are configurable via environment variables (`KREUZBERG_MAX_REQUEST_BODY_BYTES`, `KREUZBERG_MAX_MULTIPART_FIELD_BYTES`, or legacy `KREUZBERG_MAX_UPLOAD_SIZE_MB`) or programmatically via the `ApiSizeLimits` type.

## Default Configuration

### Default Limits: 100 MB

The default configuration allows:

- **Total request body:** 100 MB (104,857,600 bytes)
- **Individual file:** 100 MB (104,857,600 bytes)

These defaults are suitable for typical document processing workloads including:
- Standard PDF documents and scanned pages
- Office documents (Word, Excel, PowerPoint)
- High-resolution images
- Single document uploads and small batches

### When to Increase

Increase limits if you need to process:

- **Large scanned document archives** (200+ MB)
- **High-resolution images** (50+ MB each)
- **Video presentations** (500+ MB)
- **Bulk batch uploads** (multiple 50 MB documents)

### When to Decrease

Decrease limits if:

- You want to enforce strict file size policies
- Your server has limited memory
- You're processing only small structured documents
- You need to rate-limit aggressive clients

## Configuration Methods

### 1. Environment Variable (Simplest)

Set the `KREUZBERG_MAX_UPLOAD_SIZE_MB` environment variable to specify limits in megabytes:

```bash title="Terminal"
# Set to 200 MB
export KREUZBERG_MAX_UPLOAD_SIZE_MB=200
kreuzberg serve -H 0.0.0.0 -p 8000

# Set to 500 MB for large documents
export KREUZBERG_MAX_UPLOAD_SIZE_MB=500
kreuzberg serve -H 0.0.0.0 -p 8000
```

### 2. Docker Compose

Configure limits in your Docker Compose setup:

```yaml title="docker-compose.yaml"
version: '3.8'
services:
  kreuzberg-api:
    image: ghcr.io/kreuzberg-dev/kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      # Set maximum upload size to 500 MB
      KREUZBERG_MAX_UPLOAD_SIZE_MB: "500"
      # Configure CORS for production
      KREUZBERG_CORS_ORIGINS: "https://myapp.com,https://api.myapp.com"
    volumes:
      - ./cache:/app/.kreuzberg
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### 3. Kubernetes Deployment

Configure size limits in your Kubernetes deployment:

```yaml title="kubernetes-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: kreuzberg
        image: ghcr.io/kreuzberg-dev/kreuzberg:latest
        env:
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "500"
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://myapp.com"
        resources:
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

### 4. Programmatic Configuration

=== "C#"

    ```csharp
    using Kreuzberg;

    // Create limits: 50 MB for both request body and individual files
    var limits = ApiSizeLimits.FromMB(50, 50);

    // Or create with custom byte values
    var customLimits = new ApiSizeLimits
    {
        MaxRequestBodyBytes = 100 * 1024 * 1024,  // 100 MB
        MaxMultipartFieldBytes = 100 * 1024 * 1024  // 100 MB
    };
    ```

=== "Go"

    ```go
    import "kreuzberg"

    // Create limits: 200 MB for both request body and individual files
    limits := kreuzberg.NewApiSizeLimits(
        200 * 1024 * 1024,  // max_request_body_bytes
        200 * 1024 * 1024,  // max_multipart_field_bytes
    )

    // Or use convenience method with MB values
    limits := kreuzberg.ApiSizeLimitsFromMB(200, 200)
    ```

=== "Java"

    ```java
    import com.kreuzberg.api.ApiSizeLimits;

    // Create limits: 200 MB for both request body and individual files
    ApiSizeLimits limits = new ApiSizeLimits(
        200 * 1024 * 1024,  // maxRequestBodyBytes
        200 * 1024 * 1024   // maxMultipartFieldBytes
    );

    // Or use convenience method with MB values
    ApiSizeLimits limits = ApiSizeLimits.fromMB(200, 200);
    ```

=== "Python"

    ```python
    from kreuzberg.api import ApiSizeLimits, create_router_with_limits
    from kreuzberg import ExtractionConfig

    # Create limits: 200 MB for both request body and individual files
    limits = ApiSizeLimits.from_mb(200, 200)

    # Or create with custom byte values
    limits = ApiSizeLimits(
        max_request_body_bytes=200 * 1024 * 1024,
        max_multipart_field_bytes=200 * 1024 * 1024
    )

    # Create router with custom limits
    config = ExtractionConfig()
    router = create_router_with_limits(config, limits)
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Create limits: 200 MB for both request body and individual files
    limits = Kreuzberg::Api::ApiSizeLimits.from_mb(200, 200)

    # Or create with custom byte values
    limits = Kreuzberg::Api::ApiSizeLimits.new(
      max_request_body_bytes: 200 * 1024 * 1024,
      max_multipart_field_bytes: 200 * 1024 * 1024
    )
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};

    #[tokio::main]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
        // Create limits: 200 MB for both request body and individual files
        let limits = ApiSizeLimits::from_mb(200, 200);

        // Or create with custom byte values
        let limits = ApiSizeLimits::new(
            200 * 1024 * 1024,  // max_request_body_bytes
            200 * 1024 * 1024,  // max_multipart_field_bytes
        );

        let config = ExtractionConfig::default();
        let router = create_router_with_limits(config, limits);

        Ok(())
    }
    ```

=== "TypeScript"

    ```typescript
    import { ApiSizeLimits, createRouterWithLimits } from 'kreuzberg';

    // Create limits: 200 MB for both request body and individual files
    const limits = ApiSizeLimits.fromMb(200, 200);

    // Or create with custom byte values
    const limits = new ApiSizeLimits({
        maxRequestBodyBytes: 200 * 1024 * 1024,
        maxMultipartFieldBytes: 200 * 1024 * 1024
    });

    // Create router with custom limits
    const router = createRouterWithLimits(config, limits);
    ```

## Configuration Scenarios

### Small Documents (Default)

For standard business documents and PDFs under 50 MB:

```bash title="Terminal"
# Use default 100 MB (no configuration needed)
kreuzberg serve -H 0.0.0.0 -p 8000
```

### Medium Documents

For typical scanned document batches and office files up to 200 MB:

```bash title="Terminal"
export KREUZBERG_MAX_UPLOAD_SIZE_MB=200
kreuzberg serve -H 0.0.0.0 -p 8000
```

### Large Scans and Archives

For high-resolution scans, video content, and large archives up to 1 GB:

```bash title="Terminal"
export KREUZBERG_MAX_UPLOAD_SIZE_MB=1000
kreuzberg serve -H 0.0.0.0 -p 8000
```

### Constrained Environments

For development environments or memory-limited servers:

```bash title="Terminal"
export KREUZBERG_MAX_UPLOAD_SIZE_MB=50
kreuzberg serve -H 0.0.0.0 -p 8000
```

## Performance Considerations

### Memory Usage

File size limits directly impact memory consumption:

- **Larger limits** require more RAM to buffer request bodies
- **Streaming extraction** processes files incrementally, reducing peak memory
- **Batch requests** with multiple files consume memory for all files simultaneously

#### Memory Impact Examples

| Upload Limit | Memory Impact | Recommended RAM |
|--------------|--------------|-----------------|
| 50 MB | ~50-100 MB per request | 512 MB |
| 100 MB (default) | ~100-200 MB per request | 1 GB |
| 500 MB | ~500 MB-1 GB per request | 2-4 GB |
| 1000 MB | ~1-2 GB per request | 4-8 GB |

### Handling Large Files

When processing very large files (multi-GB):

1. **Allocate adequate RAM** - Use the memory impact table above as a guideline
2. **Increase timeouts** - Large files take longer to upload and process
3. **Monitor concurrency** - Limit concurrent uploads to prevent memory exhaustion
4. **Use streaming** - Where possible, process files streaming to reduce memory peaks

### Docker Memory Limits

Configure Docker resource limits appropriately:

```yaml title="docker-compose.yaml"
services:
  kreuzberg-api:
    image: ghcr.io/kreuzberg-dev/kreuzberg:latest
    environment:
      KREUZBERG_MAX_UPLOAD_SIZE_MB: "500"
    deploy:
      resources:
        limits:
          memory: 4G      # Limit container to 4 GB
          cpus: '2'       # Limit to 2 CPU cores
        reservations:
          memory: 2G      # Reserve 2 GB minimum
          cpus: '1'       # Reserve 1 CPU core
```

### Reverse Proxy Configuration

When using a reverse proxy (Nginx, Caddy), ensure proxy limits match or exceed Kreuzberg's limits:

**Nginx:**

```nginx title="nginx.conf"
server {
    listen 443 ssl http2;
    server_name api.example.com;

    # Match or exceed Kreuzberg's limit
    client_max_body_size 500M;

    location / {
        proxy_pass http://kreuzberg;
        # Extended timeouts for large file processing
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
        proxy_request_buffering off;  # Stream instead of buffer
    }
}
```

**Caddy:**

```caddy title="Caddyfile"
api.example.com {
    reverse_proxy localhost:8000 {
        # Match Kreuzberg's limit
        max_body_size 500MB
        # Enable streaming for large files
        flush_interval -1
    }
}
```

## Error Handling

### Exceeding Limits

When a request exceeds configured limits, the server returns a 413 Payload Too Large error:

```bash title="Terminal"
# Try to upload a 500 MB file with 100 MB default limit
curl -F "files=@large_file_500mb.zip" http://localhost:8000/extract

# Response (HTTP 413)
HTTP/1.1 413 Payload Too Large
Content-Type: application/json

{
  "error_type": "ValidationError",
  "message": "Request body exceeds maximum allowed size",
  "status_code": 413
}
```

### Client-Side Validation

Validate file sizes before upload to provide better user experience:

=== "Python"

    ```python
    import os
    from pathlib import Path

    def validate_file_size(file_path: str, max_size_mb: int) -> bool:
        """Check if file size is within limits."""
        file_size_bytes = os.path.getsize(file_path)
        file_size_mb = file_size_bytes / (1024 * 1024)

        if file_size_mb > max_size_mb:
            print(f"File {Path(file_path).name} exceeds {max_size_mb} MB limit")
            return False
        return True

    # Validate before upload
    if validate_file_size("document.pdf", max_size_mb=100):
        # Proceed with upload
        pass
    ```

=== "TypeScript"

    ```typescript
    function validateFileSize(file: File, maxSizeMB: number): boolean {
        const fileSizeMB = file.size / (1024 * 1024);

        if (fileSizeMB > maxSizeMB) {
            console.error(`File ${file.name} exceeds ${maxSizeMB} MB limit`);
            return false;
        }
        return true;
    }

    // Validate before upload
    const fileInput = document.getElementById('fileInput') as HTMLInputElement;
    fileInput.addEventListener('change', (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file && validateFileSize(file, 100)) {
            // Proceed with upload
        }
    });
    ```

=== "Go"

    ```go
    import "os"
    import "fmt"

    func validateFileSize(filePath string, maxSizeMB int64) bool {
        fileInfo, err := os.Stat(filePath)
        if err != nil {
            return false
        }

        fileSizeMB := fileInfo.Size() / (1024 * 1024)
        if fileSizeMB > maxSizeMB {
            fmt.Printf("File exceeds %d MB limit\n", maxSizeMB)
            return false
        }
        return true
    }

    // Validate before upload
    if validateFileSize("document.pdf", 100) {
        // Proceed with upload
    }
    ```

## Troubleshooting

### "Request body exceeds maximum allowed size"

**Problem:** Upload fails with HTTP 413 error

**Solutions:**

1. **Increase limit:**
   ```bash
   export KREUZBERG_MAX_UPLOAD_SIZE_MB=500
   ```

2. **Check reverse proxy limits:**
   ```nginx
   # Nginx: ensure client_max_body_size matches or exceeds Kreuzberg limit
   client_max_body_size 500M;
   ```

3. **Validate file size before upload:**
   ```bash
   # Check actual file size
   ls -lh document.pdf
   ```

### Server crashes with large files

**Problem:** Memory exhaustion when processing large files

**Solutions:**

1. **Increase container memory:**
   ```yaml
   deploy:
     resources:
       limits:
         memory: 4G
   ```

2. **Reduce upload limit:**
   ```bash
   export KREUZBERG_MAX_UPLOAD_SIZE_MB=200
   ```

3. **Process files sequentially:**
   - Send one file per request instead of batch uploads
   - Implement request queuing at the application level

4. **Monitor memory usage:**
   ```bash
   # Docker
   docker stats kreuzberg-api

   # Kubernetes
   kubectl top pod kreuzberg-api-xxxxx
   ```

### Slow uploads

**Problem:** Large file uploads timeout

**Solutions:**

1. **Increase reverse proxy timeouts:**
   ```nginx
   proxy_read_timeout 600s;  # 10 minutes
   proxy_send_timeout 600s;
   ```

2. **Enable streaming:**
   ```nginx
   proxy_request_buffering off;
   ```

3. **Check network bandwidth:**
   - For a 500 MB file over a 10 Mbps connection: 500 MB × 8 bits/byte ÷ 10 Mbps = ~400 seconds

## Best Practices

1. **Match limits to use case:** Set limits based on your actual file sizes, not theoretical maximums
2. **Monitor and adjust:** Track actual file sizes and adjust limits quarterly
3. **Use reverse proxy buffering:** Configure reverse proxies to handle buffering, not Kreuzberg
4. **Implement client-side validation:** Validate file sizes before sending to server
5. **Plan for scaling:** Run multiple Kreuzberg instances behind a load balancer for high-throughput scenarios
6. **Set appropriate timeouts:** Increase timeouts for large files (5-10 minutes recommended)
7. **Document your limits:** Keep configuration in version control with clear documentation
8. **Test with real files:** Test with actual document types you'll process in production
9. **Monitor disk space:** Large files consume both RAM and disk (if streaming to disk)
10. **Consider compression:** If applicable, compress large document batches before upload

## See Also

- [Configuration Guide](../guides/configuration.md) - Extraction configuration options
- [API Server Guide](../guides/api-server.md) - Complete API server documentation
- [Docker Deployment](../guides/docker.md) - Docker setup and configuration
- [Performance Tuning](../guides/api-server.md#performance-tuning) - Advanced performance optimization
