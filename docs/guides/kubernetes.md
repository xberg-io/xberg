# Kubernetes Deployment

Deploy Kreuzberg to Kubernetes with proper OCR configuration, permissions, and observability.

## Requirements

- Tesseract OCR initialization via `TESSDATA_PREFIX`
- Non-root container (UID 1000, GID 1000)
- Persistent volumes for Tesseract data and cache
- Health checks and resource limits

## Quick Start

```yaml title="minimal-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
  namespace: default
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
          value: "/usr/share/tesseract-ocr/4.00/tessdata"
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
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
  namespace: default
spec:
  selector:
    app: kreuzberg
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: LoadBalancer
```

Apply:

```bash
kubectl apply -f minimal-deployment.yaml
```

## Tesseract Configuration

### Critical: TESSDATA_PREFIX

Without correct `TESSDATA_PREFIX`, OCR will silently fail:

```
Warning: Image-based extraction attempted but OCR backend not available
Falling back to non-OCR extraction
```

### Built-In Tessdata (Recommended)

Official images include tessdata at `/usr/share/tesseract-ocr/4.00/tessdata/`:

```yaml
env:
- name: TESSDATA_PREFIX
  value: "/usr/share/tesseract-ocr/4.00/tessdata"
- name: KREUZBERG_OCR_LANGUAGE
  value: "eng"
```

**Pre-installed languages:** `eng`, `spa`, `fra`, `deu`, `ita`, `por`, `chi_sim`, `chi_tra`, `jpn`, `ara`, `rus`, `hin`

### Custom Tessdata via ConfigMap

For additional languages:

```bash
kubectl create configmap tessdata \
  --from-file=/path/to/eng.traineddata \
  --from-file=/path/to/deu.traineddata \
  -n default
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

### Custom Tessdata via PVC

For large custom language sets:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: tessdata-pvc
spec:
  accessModes:
    - ReadOnlyMany
  resources:
    requests:
      storage: 1Gi
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
      - name: kreuzberg
        env:
        - name: TESSDATA_PREFIX
          value: "/var/tessdata"
        volumeMounts:
        - name: tessdata-pvc
          mountPath: /var/tessdata
      volumes:
      - name: tessdata-pvc
        persistentVolumeClaim:
          claimName: tessdata-pvc
```

### Verify Tesseract

```bash
# Check installation
kubectl exec -it deployment/kreuzberg-api -- tesseract --version

# Verify TESSDATA_PREFIX
kubectl exec -it deployment/kreuzberg-api -- printenv TESSDATA_PREFIX

# List available languages
kubectl exec -it deployment/kreuzberg-api -- tesseract --list-langs

# Check logs for OCR errors
kubectl logs deployment/kreuzberg-api | grep -i "ocr\|tessdata\|tesseract"
```

## Permissions

Kreuzberg runs as non-root user (UID 1000, GID 1000).

### Fix PVC Permissions

**Option 1: Init container**

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

**Option 2: fsGroup**

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

### Restricted Security Policy

For strict Pod Security Standards:

```yaml
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    runAsGroup: 1000
    fsGroup: 1000
    seccompProfile:
      type: RuntimeDefault
  containers:
  - name: kreuzberg
    securityContext:
      runAsUser: 1000
      runAsGroup: 1000
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
    emptyDir: {}
  - name: tmp
    emptyDir: {}
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
    failureThreshold: 30  # Allow 300s to start
```

## Logging

```yaml
env:
- name: RUST_LOG
  value: "kreuzberg=debug,warn"
```

**Log levels:** `trace`, `debug`, `info`, `warn`, `error`

```bash
# View logs
kubectl logs deployment/kreuzberg-api --tail=50

# Follow logs
kubectl logs deployment/kreuzberg-api -f

# Previous logs (if crashed)
kubectl logs deployment/kreuzberg-api --previous
```

## Common Errors

### Plugin Initialization Failed

**Symptom:**
```
[ERROR] Plugin load failed: OcrBackend not initialized
```

**Fix:**

1. Verify TESSDATA_PREFIX:
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- printenv TESSDATA_PREFIX
```

2. Check tessdata files exist:
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- ls -la /usr/share/tesseract-ocr/4.00/tessdata/
```

3. Ensure environment variable is set in manifest:
```yaml
env:
- name: TESSDATA_PREFIX
  value: "/usr/share/tesseract-ocr/4.00/tessdata"
```

### MissingDependencyError

**Symptom:**
```
[ERROR] MissingDependencyError: tesseract not found in PATH
```

**Fix:**

Verify you're using the official image:
```bash
kubectl get deployment kreuzberg-api -o jsonpath='{.spec.template.spec.containers[0].image}'
```

Should be: `ghcr.io/kreuzberg-dev/kreuzberg:latest`

### Language Not Found

**Symptom:**
```
[ERROR] Tesseract language not found: deu
```

**Fix:**

Check available languages:
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- tesseract --list-langs
```

Use pre-installed languages or mount custom tessdata via PVC.

### Permission Denied

**Symptom:**
```
[ERROR] Failed to create cache directory: Permission denied
```

**Fix:**

Use init container or fsGroup (see Permissions section).

Verify permissions:
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- ls -la /app/.kreuzberg
# Should show files owned by 1000:1000
```

### Out of Memory

**Symptom:**
```
OOMKilled
```

**Fix:**

Increase memory limits:
```yaml
resources:
  limits:
    memory: "4Gi"
```

Reduce OCR resource usage:
```yaml
env:
- name: KREUZBERG_PDF_DPI
  value: "150"
- name: KREUZBERG_OCR_LANGUAGE
  value: "eng"  # Single language
```

### Startup Probe Timeout

**Symptom:**
```
Startup probe failed 30 times, giving up
```

**Fix:**

Increase timeout:
```yaml
startupProbe:
  failureThreshold: 60  # 600s = 10 minutes
  periodSeconds: 10
```

## Production Deployment

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
  accessModes:
    - ReadWriteOnce
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
          value: "/usr/share/tesseract-ocr/4.00/tessdata"
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
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 2
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
    name: http
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

Apply:

```bash
kubectl apply -f production-deployment.yaml
kubectl get deployment -n kreuzberg
kubectl get pods -n kreuzberg
kubectl get svc -n kreuzberg
```

## High Availability

```yaml title="ha-deployment.yaml"
apiVersion: v1
kind: ConfigMap
metadata:
  name: kreuzberg-config
  namespace: kreuzberg
data:
  kreuzberg.toml: |
    [ocr]
    backend = "tesseract"
    language = "eng+deu"

    [pdf]
    dpi = 300
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
  namespace: kreuzberg
spec:
  replicas: 5
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: kreuzberg
  template:
    metadata:
      labels:
        app: kreuzberg
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
                  values:
                  - kreuzberg
              topologyKey: kubernetes.io/hostname

      securityContext:
        fsGroup: 1000

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
          value: "/usr/share/tesseract-ocr/4.00/tessdata"
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://app.example.com,https://api.example.com"
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "1000"
        args: ["serve", "--host", "0.0.0.0", "--port", "8000", "--config", "/etc/kreuzberg/kreuzberg.toml"]
        resources:
          requests:
            memory: "2Gi"
            cpu: "2000m"
          limits:
            memory: "4Gi"
            cpu: "4000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 30
          timeoutSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 15
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        startupProbe:
          httpGet:
            path: /health
            port: 8000
          periodSeconds: 10
          failureThreshold: 60
        volumeMounts:
        - name: config
          mountPath: /etc/kreuzberg
        - name: cache
          mountPath: /app/.kreuzberg

      volumes:
      - name: config
        configMap:
          name: kreuzberg-config
      - name: cache
        emptyDir:
          sizeLimit: 5Gi
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
  namespace: kreuzberg
spec:
  type: ClusterIP
  clusterIP: None
  selector:
    app: kreuzberg
  ports:
  - protocol: TCP
    port: 8000
    targetPort: 8000
```

## Troubleshooting Checklist

Before reporting issues:

1. **Verify TESSDATA_PREFIX:**
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- printenv TESSDATA_PREFIX
```

2. **Check Tesseract availability:**
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- tesseract --list-langs
```

3. **Review logs:**
```bash
kubectl logs deployment/kreuzberg-api | grep -i "plugin\|ocr\|tessdata"
```

4. **Verify pod resources:**
```bash
kubectl describe pod deployment/kreuzberg-api
```

5. **Check volume permissions:**
```bash
kubectl exec -it pod/kreuzberg-api-xxx -- ls -la /app/.kreuzberg
```

6. **Test health endpoint:**
```bash
kubectl port-forward service/kreuzberg-api 8000:8000
curl http://localhost:8000/health
```

### Collect Diagnostic Information

```bash
# Logs
kubectl logs deployment/kreuzberg-api --tail=200 > logs.txt
kubectl describe deployment kreuzberg-api >> logs.txt
kubectl get events -n kreuzberg >> logs.txt

# Deployment manifest (redact secrets)
kubectl get deployment kreuzberg-api -o yaml > deployment.yaml

# Environment variables
kubectl exec -it pod/kreuzberg-api-xxx -- env | sort > env.txt
```

## Related Documentation

- [Docker Deployment](docker.md) - Container configuration
- [OCR Guide](ocr.md) - OCR backend details
- [Configuration](configuration.md) - All configuration options
- [Advanced Features](advanced.md) - Chunking, language detection
