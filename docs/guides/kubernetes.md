# Kubernetes Deployment <span class="version-badge new">v4.2.2</span>

Deploy Kreuzberg to Kubernetes with proper OCR configuration, permissions, and health checks.

## Quick Start

```yaml title="minimal-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
spec:
  replicas: 2
  selector:
    matchLabels:
      app: kreuzberg
  template:
    metadata:
      labels:
        app: kreuzberg
    spec:
      containers:
      - name: kreuzberg
        image: ghcr.io/kreuzberg-dev/kreuzberg:latest
        ports:
        - containerPort: 8000
          name: http
        env:
        - name: RUST_LOG
          value: "info"
        - name: TESSDATA_PREFIX
          value: "/usr/share/tesseract-ocr/5/tessdata"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
spec:
  selector:
    app: kreuzberg
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: LoadBalancer
```

```bash title="Terminal"
kubectl apply -f minimal-deployment.yaml
```

## Tesseract Configuration

### TESSDATA_PREFIX (Critical)

Without `TESSDATA_PREFIX`, OCR silently falls back to non-OCR extraction. Official images ship Tesseract 5.x with tessdata at `/usr/share/tesseract-ocr/5/tessdata/`.

```yaml
env:
- name: TESSDATA_PREFIX
  value: "/usr/share/tesseract-ocr/5/tessdata"
- name: KREUZBERG_OCR_LANGUAGE
  value: "eng"
- name: KREUZBERG_CACHE_DIR
  value: "/app/.kreuzberg"
- name: HF_HOME
  value: "/app/.kreuzberg/huggingface"
```

**Pre-installed languages:** `eng`, `spa`, `fra`, `deu`, `ita`, `por`, `chi_sim`, `chi_tra`, `jpn`, `ara`, `rus`, `hin`

!!! note "Tesseract Version"
    The path varies by version. Verify yours with `tesseract --version` inside the container if using a custom base image.

### Custom Languages via ConfigMap

```bash title="Terminal"
kubectl create configmap tessdata \
  --from-file=/path/to/eng.traineddata \
  --from-file=/path/to/deu.traineddata
```

```yaml
spec:
  containers:
  - name: kreuzberg
    env:
    - name: TESSDATA_PREFIX
      value: "/etc/tessdata"
    volumeMounts:
    - name: tessdata
      mountPath: /etc/tessdata
  volumes:
  - name: tessdata
    configMap:
      name: tessdata
```

For large custom language sets, use a PVC instead of a ConfigMap.

### Verify Tesseract

```bash title="Terminal"
kubectl exec -it deployment/kreuzberg-api -- tesseract --version
kubectl exec -it deployment/kreuzberg-api -- tesseract --list-langs
kubectl exec -it deployment/kreuzberg-api -- printenv TESSDATA_PREFIX
```

## Permissions

Kreuzberg runs as non-root (UID 1000, GID 1000). Fix PVC permissions with either approach:

=== "Init Container"

    ```yaml
    spec:
      initContainers:
      - name: init-permissions
        image: busybox:latest
        command: ['sh', '-c', 'chown -R 1000:1000 /app/.kreuzberg']
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
      containers:
      - name: kreuzberg
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
    ```

=== "fsGroup"

    ```yaml
    spec:
      securityContext:
        fsGroup: 1000
      containers:
      - name: kreuzberg
        securityContext:
          runAsUser: 1000
          runAsGroup: 1000
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop: ["ALL"]
    ```

## Health Checks

```yaml
containers:
- name: kreuzberg
  livenessProbe:
    httpGet:
      path: /health
      port: 8000
    initialDelaySeconds: 10
    periodSeconds: 30
    timeoutSeconds: 5
    failureThreshold: 3
  readinessProbe:
    httpGet:
      path: /health
      port: 8000
    initialDelaySeconds: 5
    periodSeconds: 10
    timeoutSeconds: 3
    failureThreshold: 2
  startupProbe:
    httpGet:
      path: /health
      port: 8000
    periodSeconds: 10
    failureThreshold: 30
```

## Logging

```yaml
env:
- name: RUST_LOG
  value: "kreuzberg=debug,warn"
```

Levels: `trace`, `debug`, `info`, `warn`, `error`

```bash title="Terminal"
kubectl logs deployment/kreuzberg-api --tail=50
kubectl logs deployment/kreuzberg-api -f
kubectl logs deployment/kreuzberg-api --previous
```

## Production Deployment

Full production manifest with namespace, PVC, security context, init container, PDB, and all probes:

```yaml title="production-deployment.yaml"
apiVersion: v1
kind: Namespace
metadata:
  name: kreuzberg
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: kreuzberg-cache
  namespace: kreuzberg
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 2Gi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
  namespace: kreuzberg
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kreuzberg
  template:
    metadata:
      labels:
        app: kreuzberg
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
        seccompProfile:
          type: RuntimeDefault
      initContainers:
      - name: init-cache
        image: busybox:latest
        command: ['sh', '-c', 'mkdir -p /app/.kreuzberg && chown -R 1000:1000 /app/.kreuzberg']
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
      containers:
      - name: kreuzberg
        image: ghcr.io/kreuzberg-dev/kreuzberg:latest
        ports:
        - containerPort: 8000
          name: http
        env:
        - name: RUST_LOG
          value: "info"
        - name: TESSDATA_PREFIX
          value: "/usr/share/tesseract-ocr/5/tessdata"
        - name: KREUZBERG_CACHE_DIR
          value: "/app/.kreuzberg"
        - name: HF_HOME
          value: "/app/.kreuzberg/huggingface"
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://app.example.com"
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "500"
        args: ["serve", "--host", "0.0.0.0", "--port", "8000"]
        resources:
          requests:
            memory: "1Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 15
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 10
        startupProbe:
          httpGet:
            path: /health
            port: 8000
          periodSeconds: 10
          failureThreshold: 30
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop: ["ALL"]
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
        - name: tmp
          mountPath: /tmp
      volumes:
      - name: cache
        persistentVolumeClaim:
          claimName: kreuzberg-cache
      - name: tmp
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
  namespace: kreuzberg
spec:
  type: LoadBalancer
  selector:
    app: kreuzberg
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
---
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: kreuzberg-pdb
  namespace: kreuzberg
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app: kreuzberg
```

```bash title="Terminal"
kubectl apply -f production-deployment.yaml
```

!!! note "Model Persistence"
    Embedding models download on first use (~90 MB – 1.2 GB). Use a PVC for `/app/.kreuzberg` to avoid re-downloading on pod restart.

## High Availability

For HA deployments, add pod anti-affinity, rolling update strategy, and a ConfigMap for extraction settings:

```yaml title="ha-additions.yaml"
spec:
  replicas: 5
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values: [kreuzberg]
              topologyKey: kubernetes.io/hostname
```

## Troubleshooting

??? question "OCR silently failing"

    Verify `TESSDATA_PREFIX` is set and tessdata files exist:

    ```bash title="Terminal"
    kubectl exec -it deployment/kreuzberg-api -- printenv TESSDATA_PREFIX
    kubectl exec -it deployment/kreuzberg-api -- ls /usr/share/tesseract-ocr/5/tessdata/
    ```

??? question "Permission denied on cache directory"

    Use an init container or `fsGroup` (see [Permissions](#permissions)).

??? question "OOMKilled"

    Increase memory limits. Reduce OCR resource usage with `KREUZBERG_PDF_DPI=150` and single-language OCR.

??? question "Startup probe timeout"

    Increase `failureThreshold` on the startup probe (e.g., `60` for 10-minute timeout).

??? question "Language not found"

    Check installed languages with `kubectl exec -it deployment/kreuzberg-api -- tesseract --list-langs`. Mount custom tessdata via ConfigMap or PVC.

### Diagnostic Commands

```bash title="Terminal"
kubectl logs deployment/kreuzberg-api --tail=200
kubectl describe deployment kreuzberg-api
kubectl get events -n kreuzberg
kubectl exec -it deployment/kreuzberg-api -- env | sort
kubectl port-forward service/kreuzberg-api 8000:8000 && curl http://localhost:8000/health
```

## Next Steps

- [Docker Deployment](docker.md) — container configuration and image variants
- [API Server Guide](api-server.md) — endpoint documentation
- [OCR Guide](ocr.md) — backend installation and language setup
- [Configuration](configuration.md) — all configuration options
