---
title: "Kubernetes Deployment"
description: "Deploy the Xberg extraction server on Kubernetes with the official Helm chart."
---

Deploy the Xberg REST API server (`xberg serve`) on Kubernetes with the official Helm chart. The chart is a single-service deployment — one stateless workload plus an optional cache, ingress, and autoscaler. For a distributed, governed platform with team support, see [Xberg Enterprise](https://pro.xberg.io/start/pro-helm/).

## Install

The chart is published as an OCI artifact to GitHub Container Registry:

```bash title="Terminal"
helm install xberg oci://ghcr.io/xberg-io/charts/xberg --version 1.2.5
```

This runs the full image (`ghcr.io/xberg-io/xberg`) in API-server mode on port 8000, exposed through a ClusterIP `Service` on port 80.

The chart is also listed on [Artifact Hub](https://artifacthub.io/packages/helm/xberg-core/xberg), where you can browse every version, its values, and the generated values schema.

## Verify the chart

Every published chart is signed with [cosign](https://docs.sigstore.dev/) using keyless signing (Sigstore OIDC + the Rekor transparency log). Verify a release before installing:

```bash title="Terminal"
cosign verify \
  ghcr.io/xberg-io/charts/xberg:1.2.5 \
  --certificate-identity-regexp '^https://github.com/xberg-io/xberg/.github/workflows/publish-helm.yaml@.*$' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

The identity certificate ties each signature to the `publish-helm.yaml` workflow in this repository, so a successful verification proves the chart was built and pushed by Xberg's release pipeline.

## Configure

Override defaults with a `values.yaml` file:

```yaml title="values.yaml"
image:
  # Empty tag defaults to the chart appVersion. Use "core" for the minimal
  # image (no pre-downloaded models) or "latest" for the full image.
  tag: ""

xberg:
  logLevel: "info"
  ocrLanguage: "eng"

resources:
  requests:
    memory: "1Gi"
    cpu: "1000m"
  limits:
    memory: "4Gi"
    cpu: "2000m"

ingress:
  enabled: true
  className: "nginx"
  hosts:
    - host: xberg.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: xberg-tls
      hosts:
        - xberg.example.com

autoscaling:
  enabled: true
  minReplicas: 1
  maxReplicas: 10
  targetCPUUtilizationPercentage: 80
```

```bash title="Terminal"
helm install xberg oci://ghcr.io/xberg-io/charts/xberg \
  --version 1.2.5 \
  -f values.yaml
```

## Cache and replicas

Embedding and OCR models range from ~90 MB to 1.2 GB and are re-downloaded on every pod restart without a cache. The chart enables a `ReadWriteOnce` PVC (`cache.enabled: true`) mounted at `/app/.xberg` (with `HF_HOME` under it) to persist them.

:::caution
A `ReadWriteOnce` volume can only attach to one node, so the chart defaults to `replicaCount: 1` with a `Recreate` strategy. To run multiple replicas, either switch the cache to `ReadWriteMany` storage or set `cache.enabled: false` (each pod then re-downloads models into an ephemeral volume).
:::

## Upgrade and uninstall

```bash title="Terminal"
helm upgrade xberg oci://ghcr.io/xberg-io/charts/xberg --version 1.2.5 -f values.yaml
helm uninstall xberg
```

The cache PVC carries `helm.sh/resource-policy: keep`, so it survives an uninstall — delete it manually if you no longer need the cached models.

## What's included

| Resource | Description | Conditional |
|----------|-------------|-------------|
| Deployment | API server with health probes and a hardened, non-root, read-only-root security context | Always |
| Service | ClusterIP on port 80 → container 8000 | Always |
| ServiceAccount | Dedicated service account | `serviceAccount.create` |
| PersistentVolumeClaim | Cache for models and downloaded assets | `cache.enabled` |
| Ingress | HTTP(S) ingress with optional TLS | `ingress.enabled` |
| HorizontalPodAutoscaler | CPU/memory-based autoscaling | `autoscaling.enabled` |
| PodDisruptionBudget | Availability during voluntary disruptions | `podDisruptionBudget.enabled` |

All values are documented in the chart's [`values.yaml`](https://github.com/xberg-io/xberg/blob/main/charts/xberg/values.yaml) and validated on install against the bundled [`values.schema.json`](https://github.com/xberg-io/xberg/blob/main/charts/xberg/values.schema.json), so a malformed override fails fast with a clear error.

## Next steps

- [Docker Deployment](/guides/docker/) — image variants and execution modes
- [API Server](/guides/api-server/) — endpoint reference
- [OCR](/guides/ocr/) — backends and language configuration
