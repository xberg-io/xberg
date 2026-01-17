```bash title="Bash"
docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest extract /data/document.pdf

```
