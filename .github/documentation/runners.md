# Custom GitHub Actions Runners

## Available Runners

| Runner Label | Architecture | Size | Ephemeral | Notes |
|---|---|---|---|---|
| `runner-small` | x86_64 | Small | No | Light tasks: linting, formatting, validation |
| `runner-medium` | x86_64 | Medium | No | Standard CI: tests, builds |
| `runner-medium-arm64` | arm64 | Medium | No | ARM64 builds and tests |
| `runner-large` | x86_64 | Large | No | Heavy workloads: benchmarks, coverage, release builds |
| `runner-large-spot` | x86_64 | Large | Yes | Cost-optimized large jobs where interruption is acceptable |
| `runner-medium-arm64-spot` | arm64 | Medium | Yes | Cost-optimized ARM64 jobs where interruption is acceptable |

## Spot Runners

Spot runners (`*-spot`) use ephemeral cloud instances provisioned on a best-effort basis. They are significantly cheaper but can be preempted at any time if the cloud provider reclaims capacity.

**Use spot runners for:**

- Jobs that can be retried without consequence (test suites, linting)
- Non-time-critical workloads
- PR validation where re-runs are acceptable

**Do not use spot runners for:**

- Benchmarks (preemption and noisy-neighbor effects skew results)
- Release builds and publishing
- Jobs requiring consistent, reproducible timing

## Choosing a Runner

| Workload | Recommended Runner |
|---|---|
| Linting, formatting, validation | `runner-small` |
| Unit tests, standard builds | `runner-medium` |
| ARM64 cross-compilation / tests | `runner-medium-arm64` |
| Benchmarks, coverage reports | `runner-large` |
| Non-critical large builds | `runner-large-spot` |
| Non-critical ARM64 builds | `runner-medium-arm64-spot` |
