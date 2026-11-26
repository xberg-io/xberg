# Xberg Helm chart

Deploy the [Xberg](https://github.com/xberg-io/xberg) document-intelligence server
(HTTP API + MCP) on Kubernetes. Extract text, tables, metadata, and structured data
from PDFs, Office documents, images, and 107 formats, with optional OCR.

The chart is published as an OCI artifact to GHCR.

## Install

```bash
helm install xberg oci://ghcr.io/xberg-io/charts/xberg --version 1.5.4
```

Override values inline or with a file:

```bash
helm install xberg oci://ghcr.io/xberg-io/charts/xberg \
  --version 1.5.4 \
  --set service.type=LoadBalancer \
  --set cache.size=5Gi
```

## Configuration

Values are validated against `values.schema.json`. Common keys:

| Key                  | Default            | Description                                              |
| -------------------- | ------------------ | ------------------------------------------------------- |
| `replicaCount`       | `1`                | Replicas. Keep at 1 with ReadWriteOnce cache storage.   |
| `image.repository`   | `xberg-io/xberg`   | Image repository (`ghcr.io` registry).                  |
| `image.tag`          | `""`               | Defaults to the chart `appVersion`.                     |
| `service.type`       | `ClusterIP`        | Service type.                                            |
| `service.port`       | `80`               | Service port.                                            |
| `ingress.enabled`     | `false`            | Enable Ingress.                                          |
| `ingress.extraLabels` | `{}`               | Extra labels on the Ingress resource.                    |
| `ingress.annotations` | `{}`               | Annotations on the Ingress resource.                     |
| `resources`          | 512Mi/500m → 2Gi/2 | Container requests and limits.                           |
| `autoscaling.enabled`| `false`            | Enable a HorizontalPodAutoscaler.                        |
| `xberg.logLevel`     | `info`             | Log level: trace, debug, info, warn, error.             |
| `xberg.ocrLanguage`  | `eng`              | Default OCR language.                                    |
| `xberg.tessdataPrefix` | `""`             | `TESSDATA_PREFIX`; empty leaves it unset. Must be empty with best models. |
| `ocr.tesseractModelSet` | `fast`          | Tesseract models: `fast` or `best` (`XBERG_OCR_TESSDATA_MODEL`). |
| `ocr.tesseractModelSource` | `image`      | With `best`: `image` runs the `-best` image tag, `download` fetches models into the cache. |
| `xberg.apiTokenSecret.name` | `""`        | Existing Secret with the API bearer token (`XBERG_API_TOKEN`); required unless `allowUnauthenticated` is true. |
| `xberg.apiTokenSecret.key`  | `token`     | Key in that Secret.                                      |
| `xberg.allowUnauthenticated` | `false`    | Start without a token (`XBERG_ALLOW_UNAUTHENTICATED`).   |
| `cache.enabled`      | `true`             | Persist downloaded models on a PVC (90 MB–1.2 GB each). |
| `cache.size`         | `2Gi`              | Cache PVC size.                                          |
| `cache.seedFromImage` | `true`            | Copy the image's `/app/.xberg` models into the cache volume at pod start. |

See [`values.yaml`](values.yaml) for the full, commented list.

## Tesseract models

`ocr.tesseractModelSet: best` switches OCR to `tessdata_best`: more accurate, several times
slower per page, and more memory per OCR request than the default `tessdata_fast`, so review
`resources.limits.memory` before turning it on. With `ocr.tesseractModelSource: image` (the
default) the chart runs the `-best` twin of the image tag, which ships the best models for the
image's OCR languages: an empty tag becomes `<appVersion>-best`, `latest` becomes `latest-best`
and `core` becomes `core-best`. With `download` the tag is kept and each best model is downloaded
into the cache volume on first use, which needs egress to `raw.githubusercontent.com`.

`xberg.tessdataPrefix` sets `TESSDATA_PREFIX`, which xberg uses as given for every model set, so
the chart rejects it with `best`. It defaults to empty: xberg finds the image's tessdata without
it, so the default `fast` deployment uses the same `tessdata_fast` models as before.

## Cache

Embedding, OCR, and layout models range from 90 MB to 1.2 GB and are re-downloaded
on every pod restart without a persistent cache. `cache.enabled=true` (the default)
mounts a PVC to keep them across restarts. With `ReadWriteOnce` storage, keep
`replicaCount: 1` and `strategy.type: Recreate` to avoid Multi-Attach errors.

The cache volume is mounted at `/app/.xberg`, where the full image ships its pre-downloaded
PaddleOCR and layout models. `cache.seedFromImage=true` (the default) runs a `seed-cache` init
container from the same image that copies those files into the volume, keeping files the volume
already holds, so the models are not downloaded again and a PVC is only filled on its first
start. Without a PVC (`cache.enabled=false`) the emptyDir starts empty on every pod, so each
start copies the full image's ~1 GB of models and needs that much ephemeral storage on the node.
Use the PVC (the default) with the full image; with the core image, which ships no models, set
`cache.seedFromImage=false`.

## Documentation

Full deployment guide: <https://docs.xberg.io/guides/kubernetes/>

## License

MIT — see the [Xberg repository](https://github.com/xberg-io/xberg).
