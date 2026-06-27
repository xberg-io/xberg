```python title="usage.py"
import subprocess
import httpx
import json
from pathlib import Path

class DockerXbergClient:
    def __init__(self, container_name: str = "xberg-api", port: int = 8000):
        self.container_name = container_name
        self.port = port
        self.api_url = f"http://localhost:{port}/api/extract"

    def start_container(self, image: str = "xberg:latest"):
        print("Starting Xberg Docker container...")
        subprocess.run(
            [
                "docker", "run", "-d",
                "--name", self.container_name,
                "-p", f"{self.port}:8000",
                image,
            ],
            check=True,
        )
        print(f"Container started on http://localhost:{self.port}")

    async def extract(self, file_path: str) -> str:
        file_bytes = Path(file_path).read_bytes()
        files = {"file": (Path(file_path).name, file_bytes)}

        async with httpx.AsyncClient() as client:
            response = await client.post(self.api_url, files=files)
            response.raise_for_status()
            result = response.json()
            return result.get("content", "")

    def stop_container(self):
        print("Stopping Xberg Docker container...")
        subprocess.run(["docker", "stop", self.container_name], check=True)
        subprocess.run(["docker", "rm", self.container_name], check=True)
        print("Container stopped and removed")

async def main():
    docker_client = DockerXbergClient()

    try:
        docker_client.start_container()
        import asyncio
        await asyncio.sleep(2)

        content = await docker_client.extract("document.pdf")
        print(f"Extracted content:\n{content}")
    finally:
        docker_client.stop_container()

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
```
