# Docker Configuration Testing Scripts

This directory contains comprehensive testing scripts for validating Docker configuration scenarios.

## Scripts

### test-docker-config-local.sh

A comprehensive local Docker testing script that validates all configuration volume mount scenarios.

#### Purpose

Tests Docker configuration in various scenarios:
- Volume mounts to `/etc/kreuzberg/kreuzberg.toml` (recommended system path)
- Volume mounts to `/app/.config/kreuzberg/config.toml` (user path)
- Custom paths with `--config` flag
- Environment variable overrides with config files
- All config formats (TOML, YAML, JSON)
- Read-only mounts (`:ro` flag)

#### Requirements

- Docker installed and running
- Docker images pre-built (`kreuzberg:core` and/or `kreuzberg:full`)
- Port range 18100-18199 available for testing

#### Usage

```bash
./test-docker-config-local.sh [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `--variant VARIANT` | Test specific variant: `core`, `full`, or `all` | `all` |
| `--verbose` | Enable verbose debugging output | Disabled |
| `--keep-containers` | Preserve containers after tests for inspection | Clean up |
| `--help` | Display help message | - |

#### Examples

Test both core and full variants:
```bash
./test-docker-config-local.sh
```

Test only the full variant with verbose output:
```bash
./test-docker-config-local.sh --variant full --verbose
```

Test core variant and keep containers for inspection:
```bash
./test-docker-config-local.sh --variant core --keep-containers
```

#### Test Cases

The script runs 8 test cases for each variant:

1. **Volume mount to /etc/kreuzberg/kreuzberg.toml**
   - Tests the recommended system-wide configuration path
   - Validates read-only mount functionality

2. **Volume mount to /app/.config/kreuzberg/config.toml**
   - Tests the user-level configuration path
   - Validates alternative mount location

3. **Custom path with --config flag**
   - Tests custom configuration file paths
   - Validates explicit path specification via CLI flag

4. **Environment variable overrides with config file**
   - Tests that environment variables can override config file settings
   - Validates configuration precedence

5. **TOML config format**
   - Tests TOML configuration file format support
   - Validates parsing of TOML syntax

6. **YAML config format**
   - Tests YAML configuration file format support
   - Validates parsing of YAML syntax

7. **JSON config format**
   - Tests JSON configuration file format support
   - Validates parsing of JSON syntax

8. **Read-only mount**
   - Tests that containers work correctly with read-only mounts
   - Validates security of mounted volumes

#### Validation Method

For each test, the script:
1. Creates a temporary configuration file in the specified format
2. Starts a Docker container with the configuration mounted
3. Waits for the service to become healthy (up to 30 seconds)
4. Verifies the health endpoint responds successfully
5. Stops and removes the container
6. Reports pass/fail status

#### Output

The script provides clear, color-coded output:
- `[PASS]` - Test passed (green)
- `[FAIL]` - Test failed (red)
- `[INFO]` - Informational messages (blue)
- `[WARN]` - Warnings (yellow)
- `[DEBUG]` - Debug information (yellow, with `--verbose`)

Example output:
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

Test 02: Volume mount to /app/.config/kreuzberg/config.toml (variant: core)
[PASS] Test passed

...

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

#### Troubleshooting

**Error: Docker is not installed or not in PATH**
- Install Docker from https://www.docker.com/products/docker-desktop
- Ensure Docker is in your system PATH

**Error: Docker daemon is not running**
- Start Docker Desktop or the Docker daemon
- On Linux: `sudo systemctl start docker`

**Error: Docker image does not exist**
- Build the required image(s):
  ```bash
  cd /path/to/kreuzberg
  docker build -f docker/Dockerfile.core -t kreuzberg:core .
  docker build -f docker/Dockerfile.full -t kreuzberg:full .
  ```

**Tests timing out**
- Check system resources (CPU, memory)
- Increase timeout: Modify `TIMEOUT_SECONDS=30` in the script
- Check Docker logs: `docker logs <container-name>`

**Port conflicts**
- Ensure ports 18100-18199 are available
- Check for existing containers: `docker ps -a`
- Kill conflicting containers: `docker kill <container-name>`

#### Environment Variables

The script respects these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `TEST_VARIANT` | Override variant via environment | Unset |
| `VERBOSE` | Enable verbose output via environment | `false` |
| `KEEP_CONTAINERS` | Keep containers via environment | `false` |

Example:
```bash
VERBOSE=true ./test-docker-config-local.sh --variant core
```

#### Temporary Files

The script creates temporary configuration files in `/tmp/kreuzberg-config-test-$PID/`:
- `kreuzberg.toml` - TOML format test config
- `config.yaml` - YAML format test config
- `config.json` - JSON format test config

These are automatically cleaned up after tests complete (unless `--keep-containers` is used).

#### Exit Codes

- `0` - All tests passed
- `1` - One or more tests failed, or Docker is not available

#### Performance Notes

- Each test takes approximately 2-5 seconds
- Total test suite runtime: 1-2 minutes for all variants
- Network latency may affect health check timing
- Container startup time depends on system resources

#### CI/CD Integration

The script can be integrated into CI/CD pipelines:

```bash
#!/bin/bash
set -e

# Build images
docker build -f docker/Dockerfile.core -t kreuzberg:core .
docker build -f docker/Dockerfile.full -t kreuzberg:full .

# Run tests
./scripts/test/test-docker-config-local.sh --variant all

echo "Configuration tests passed!"
```

#### Limitations

- Requires Docker to be installed and running
- Tests only configuration volume mounts (not other volume types)
- Tests only health endpoint (basic connectivity validation)
- Assumes `kreuzberg:*` image naming convention
- Tests run sequentially (not parallelized)

#### Future Enhancements

Potential improvements:
- Parallel test execution for faster results
- Additional validation endpoints (beyond `/health`)
- Configuration value verification (test that config was actually loaded)
- Performance benchmarking
- Multi-architecture testing (arm64, amd64)
- Docker Compose integration tests
