# Host identity and load at run time

A shared workstation, not an isolated benchmarking host. Every timing number in this directory is
observed wall-clock time under other load. **No cold-load, warm p50/p95, throughput or peak-RSS
claim is made**, and none should be quoted as a resource budget.

| field | value |
| --- | --- |
| CPU | Apple M4 Pro (recorded from `sysctl -n machdep.cpu.brand_string`) |
| memory | 48 GiB |
| OS | macOS 27.2.1 (`sw_vers -productVersion`) |
| arch | aarch64 |
| ONNX Runtime | 1.24.2, `ORT Build Info: git-branch=HEAD, git-commit-id=058787ceea, build type=Release` (NuGet `Microsoft.ML.OnnxRuntime` 1.24.2 osx-arm64, dynamically linked) |
| ort crate | 2.0.0-rc.15, `api-18` |
| threads | 0, meaning the SDK default; the SDK session-thread setter is private and the runner is serial |
| runner sha256 | `0755ab073af1c255f48eb246a3999ff4861fdcbc5d56cb97a0307e3ec566d189` |
| dependency lock sha256 | `e53173ffc9fb1ba1d5e1479c9c80d4478f7d05c408500f1ae8d9a16ec2257616` |

## Load

| run | window (local) | `uptime` load averages |
| --- | --- | --- |
| classification (819 inferences) | 19:53–20:00 | 24.16, 31.46, 34.04 at start |
| captioning (120 generations) | 20:00–20:07 | 31.87, 30.00, 31.51 at end |

Other builds and agents were active throughout. The load figures are recorded rather than argued
away; the runs were executed because the quality metrics are deterministic and load-insensitive,
but the timing figures are context only.

## Peak RSS

Not sampled. `sysinfo`'s vendored override is present in the workspace but this package does not
sample memory. A resource budget needs an exclusive host, RSS sampling and a cold/warm split.
