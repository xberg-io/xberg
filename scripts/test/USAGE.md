# Docker Configuration Testing - Quick Start Guide

## Overview

The `test-docker-config-local.sh` script provides comprehensive testing for Docker configuration volume mounts and environment variable overrides.

## Prerequisites

1. **Docker**: Installed and running
2. **Images**: Pre-built Docker images for testing
3. **Ports**: 18100-18199 available for test containers
4. **Utilities**: `bash`, `curl`, `docker` command-line tools

## Building Test Images

Before running tests, build the Docker images:

```bash
cd .

# Build core variant
docker build -f docker/Dockerfile.core -t kreuzberg:core .

# Build full variant
docker build -f docker/Dockerfile.full -t kreuzberg:full .

# Or build both
docker build -f docker/Dockerfile.core -t kreuzberg:core . && \
docker build -f docker/Dockerfile.full -t kreuzberg:full .
```

## Running Tests

### Basic Usage

Test all variants with default settings:

```bash
./scripts/test/test-docker-config-local.sh
```

### Common Commands

**Test only core variant:**
```bash
./scripts/test/test-docker-config-local.sh --variant core
```

**Test only full variant:**
```bash
./scripts/test/test-docker-config-local.sh --variant full
```

**Enable verbose output:**
```bash
./scripts/test/test-docker-config-local.sh --verbose
```

**Keep containers after testing:**
```bash
./scripts/test/test-docker-config-local.sh --keep-containers
```

**Combine multiple options:**
```bash
./scripts/test/test-docker-config-local.sh --variant full --verbose --keep-containers
```

## Test Cases Explained

### 1. Volume Mount to /etc/kreuzberg/kreuzberg.toml

**What it tests**: System-wide configuration path (recommended)

**Docker command**:
```bash
docker run -v /local/config.toml:/etc/kreuzberg/kreuzberg.toml:ro kreuzberg:full
```

**Expected**: Container reads config from standard system location

---

### 2. Volume Mount to /app/.config/kreuzberg/config.toml

**What it tests**: User-level configuration path (alternative location)

**Docker command**:
```bash
docker run -v /local/config.toml:/app/.config/kreuzberg/config.toml:ro kreuzberg:full
```

**Expected**: Container reads config from user application directory

---

### 3. Custom Path with --config Flag

**What it tests**: Explicit configuration path specification

**Docker command**:
```bash
docker run \
  -v /local/config.toml:/app/custom-config.toml:ro \
  --entrypoint "/app/kreuzberg" \
  kreuzberg:full \
  --config /app/custom-config.toml
```

**Expected**: Container uses specified custom path

---

### 4. Environment Variable Overrides

**What it tests**: Environment variables override config file settings

**Docker command**:
```bash
docker run \
  -v /local/config.toml:/etc/kreuzberg/kreuzberg.toml:ro \
  -e KREUZBERG_SERVER_PORT=8000 \
  kreuzberg:full
```

**Expected**: Environment variable takes precedence over config file

---

### 5. TOML Format Support

**What it tests**: Configuration in TOML format

**Config file**:
```toml
[server]
host = "0.0.0.0"
port = 8000
max_upload_mb = 100

[ocr]
backend = "tesseract"
language = "eng"
```

**Expected**: Container parses TOML correctly

---

### 6. YAML Format Support

**What it tests**: Configuration in YAML format

**Config file**:
```yaml
server:
  host: "0.0.0.0"
  port: 8000
  max_upload_mb: 100

ocr:
  backend: "tesseract"
  language: "eng"
```

**Expected**: Container parses YAML correctly

---

### 7. JSON Format Support

**What it tests**: Configuration in JSON format

**Config file**:
```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8000,
    "max_upload_mb": 100
  },
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}
```

**Expected**: Container parses JSON correctly

---

### 8. Read-Only Mount

**What it tests**: Security of read-only mounted volumes

**Docker command**:
```bash
docker run -v /local/config.toml:/etc/kreuzberg/kreuzberg.toml:ro kreuzberg:full
```

**Expected**: Container works with read-only volumes, application doesn't attempt to modify config

---

## Understanding Output

### Success Output

```
╔════════════════════════════════════════════════════════╗
║ Docker Configuration Volume Mount Test Suite           ║
╚════════════════════════════════════════════════════════╝

[INFO] Configuration:
[INFO]   Variant:         all
[INFO]   Verbose:         false
[INFO]   Keep Containers: false
[INFO]   Port Range:      18100-18199

[INFO] Docker is available

Test 01: Volume mount to /etc/kreuzberg/kreuzberg.toml (variant: core)
[PASS] Test passed
```

### Failure Output

```
Test 02: Custom path with --config flag (variant: core)
[FAIL] Test failed: Failed to start container with custom --config flag
[FAIL]   Details: Container logs:
          /app/kreuzberg: line 123: syntax error: unexpected token
```

### Summary

```
╔════════════════════════════════════════════════════════╗
║ Test Summary                                           ║
╚════════════════════════════════════════════════════════╝

Total Tests:   16
Passed Tests:  16
Failed Tests:  0
Pass Rate:     100%

Tested Variants:
  - kreuzberg:core
  - kreuzberg:full
```

## Debugging Failed Tests

### Enable Verbose Output

```bash
./scripts/test/test-docker-config-local.sh --variant core --verbose
```

Verbose output shows:
- Container IDs
- Docker arguments
- Service startup timing
- Health check attempts

### Keep Containers for Inspection

```bash
./scripts/test/test-docker-config-local.sh --keep-containers
```

Then inspect containers manually:

```bash
# List test containers
docker ps -a | grep kreuzberg-config-test

# View specific container logs
docker logs kreuzberg-config-test-etc-core-12345

# Execute command in running container
docker exec kreuzberg-config-test-etc-core-12345 cat /etc/kreuzberg/kreuzberg.toml

# Stop container manually
docker stop kreuzberg-config-test-etc-core-12345
docker rm kreuzberg-config-test-etc-core-12345
```

### Check Health Endpoint Manually

```bash
# Start container manually
docker run -d \
  --name test-container \
  -p 8000:8000 \
  -v /path/to/config.toml:/etc/kreuzberg/kreuzberg.toml:ro \
  kreuzberg:full

# Wait for startup
sleep 3

# Test health endpoint
curl -v http://localhost:8000/health

# View logs
docker logs test-container

# Cleanup
docker stop test-container
docker rm test-container
```

## Troubleshooting

### Docker Not Found

```
[ERROR] Docker is not installed or not in PATH
```

**Solution**: Install Docker or ensure it's in your PATH

```bash
which docker
export PATH=$PATH:/usr/local/bin  # or wherever docker is installed
```

### Docker Daemon Not Running

```
[ERROR] Docker daemon is not running or you don't have permissions
```

**Solution**: Start Docker

```bash
# macOS
open -a Docker

# Linux
sudo systemctl start docker

# Check status
docker ps
```

### Image Not Found

```
[WARN] Skipping tests for variant: full (image not found)
```

**Solution**: Build the image

```bash
docker build -f docker/Dockerfile.full -t kreuzberg:full .
```

### Port Already in Use

```
[FAIL] Test failed: Failed to start container
[FAIL]   Details: port is already allocated
```

**Solution**: Free the ports or wait for existing tests to finish

```bash
# Find what's using the ports
lsof -i :18100-18199

# Or just stop all test containers
docker ps -a --filter "name=kreuzberg-config-test" --format "{{.Names}}" | \
  xargs -r docker stop
```

### Health Check Timeout

```
[FAIL] Test failed: Service failed to start (health check timeout)
```

**Debugging**:

1. Check container is still running:
```bash
docker ps | grep kreuzberg-config-test
```

2. View container logs:
```bash
docker logs <container-name>
```

3. Check if service is binding to port:
```bash
docker exec <container-name> netstat -tuln | grep 8000
```

4. Increase timeout (edit script):
```bash
TIMEOUT_SECONDS=60  # Change from 30
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Docker Config Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker images
        run: |
          docker build -f docker/Dockerfile.core -t kreuzberg:core .
          docker build -f docker/Dockerfile.full -t kreuzberg:full .

      - name: Run configuration tests
        run: ./scripts/test/test-docker-config-local.sh --variant all
```

### GitLab CI

```yaml
docker-config-tests:
  stage: test
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -f docker/Dockerfile.core -t kreuzberg:core .
    - docker build -f docker/Dockerfile.full -t kreuzberg:full .
    - ./scripts/test/test-docker-config-local.sh --variant all
```

## Performance Expectations

| Metric | Time |
|--------|------|
| Single test | 2-5 seconds |
| All 8 tests (1 variant) | 30-45 seconds |
| All 16 tests (2 variants) | 60-90 seconds |
| With verbose output | +10-20 seconds |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed OR Docker unavailable |

## Advanced Usage

### Custom Environment Variables

```bash
# Override variant via environment
TEST_VARIANT=core ./scripts/test/test-docker-config-local.sh

# Override verbose via environment
VERBOSE=true ./scripts/test/test-docker-config-local.sh
```

### Modify Timeout

Edit the script to change timeout:

```bash
TIMEOUT_SECONDS=60  # Line ~43, change from 30
```

### Test Specific Scenarios

To test only one specific scenario, modify the `run_test_suite()` call in `main()`:

```bash
# Comment out unwanted tests
# test_etc_kreuzberg_mount "$variant"
test_app_config_mount "$variant"
# test_custom_path_with_flag "$variant"
# ... etc
```

## Getting Help

```bash
./scripts/test/test-docker-config-local.sh --help
```

For detailed documentation:

```bash
cat ./scripts/test/README.md
```

## Related Files

- **Script**: `./scripts/test/test-docker-config-local.sh`
- **Documentation**: `./scripts/test/README.md`
- **This Guide**: `./scripts/test/USAGE.md`
- **Docker Files**: `./docker/Dockerfile.core`
- **Docker Files**: `./docker/Dockerfile.full`
