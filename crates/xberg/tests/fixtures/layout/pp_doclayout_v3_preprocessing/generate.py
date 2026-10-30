# /// script
# requires-python = ">=3.13,<3.14"
# dependencies = ["numpy==2.2.6", "opencv-python-headless==4.10.0.84"]
# ///
"""Regenerate synthetic OpenCV references; no model or document corpus is used."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import platform
import shutil
import subprocess
import urllib.request
from pathlib import Path

import cv2
import numpy as np

INPUT_SIZE = 800
PADDLEX_REVISION = "4c968d30b9d0d4a693d9eaaef0881c20f8d7d0e0"
MODEL_REVISION = "241f8bdfc77a7c7bee915a5057aaee58c235a8d3"
MODEL_URL = f"https://huggingface.co/PaddlePaddle/PP-DocLayoutV3/resolve/{MODEL_REVISION}/inference.yml"
MODEL_SHA256 = "506fcfac13b3b546ae40d7886b44126420f392adb694e3f8bb6a6286a1f90fdc"
SOURCES = {
    "paddlex/inference/models/layout_analysis/predictor.py": "e08b4347d79c029da1a4b592808629fb4a88f2a63e747ac15410191e4ff3ddf6",
    "paddlex/inference/models/object_detection/processors.py": "de2bb4f14f01619435c65df09272d978619c045c915cba465990dc4c32ac3540",
    "paddlex/inference/models/common/vision/processors.py": "e6010780c1221990c989a1ad599b39bc471b96c286c35af2200b9e0552a66063",
    "paddlex/inference/models/common/vision/funcs.py": "fa3d0c3bbc6d5cf016da27132eac4cabb089d1a96fc706984776c805d6237fb6",
    "paddlex/inference/models/object_detection/predictor.py": "e7b1421d33c3f78d20e820c92df8c766ad978b8dbaeac8f0fdd1bbc3b1a9581e",
}
CASES = (
    ("odd_portrait", 37, 59, False, False),
    ("odd_landscape", 1603, 901, False, False),
    ("rotated_portrait", 37, 59, True, False),
    ("saturated_edges", 13, 11, False, True),
    ("identity", 800, 800, False, False),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def synthetic_rgb(width: int, height: int, *, edges: bool) -> np.ndarray:
    y, x = np.indices((height, width), dtype=np.uint32)
    if edges:
        red = np.where(x == 0, 255, 0)
        green = np.where(y == height - 1, 255, 0)
        blue = np.where((x + y) % 2 == 0, 255, 0)
    else:
        red = np.where(x % 29 < 7, 255, 0)
        green = (17 * x + 31 * y) % 256
        blue = np.where((x // 13 + y // 11) % 2 == 0, 255, 0)
    return np.stack((red, green, blue), axis=-1).astype(np.uint8)


def source_records() -> list[dict[str, str]]:
    root = f"https://raw.githubusercontent.com/PaddlePaddle/PaddleX/{PADDLEX_REVISION}/"
    records = [{"url": root + path, "sha256": digest} for path, digest in SOURCES.items()]
    records.extend(
        [
            {"url": MODEL_URL, "sha256": MODEL_SHA256},
            {
                "url": "https://raw.githubusercontent.com/opencv/opencv/4.10.0/modules/imgproc/src/resize.cpp",
                "sha256": "60e44ab978fda269c19055659d0f1d7cdd32c1bfce8853ceb8dd6166833ee177",
            },
        ]
    )
    return records


def verify_upstream(records: list[dict[str, str]]) -> None:
    for record in records:
        with urllib.request.urlopen(record["url"], timeout=30) as response:
            actual = sha256(response.read())
        if actual != record["sha256"]:
            raise ValueError(f"Upstream source digest changed: {record['url']}")


def generate_case(case: tuple[str, int, int, bool, bool], output: Path) -> dict:
    name, width, height, rotate, edges = case
    source = synthetic_rgb(width, height, edges=edges)
    if rotate:
        source = cv2.rotate(source, cv2.ROTATE_90_CLOCKWISE)
    resized = cv2.resize(source, (INPUT_SIZE, INPUT_SIZE), interpolation=cv2.INTER_CUBIC)
    linear = cv2.resize(source, (INPUT_SIZE, INPUT_SIZE), interpolation=cv2.INTER_LINEAR)
    linear_error = int(np.abs(resized.astype(np.int16) - linear.astype(np.int16)).max())
    if name != "identity" and linear_error <= 1:
        raise ValueError(f"The cubic reference does not reject a wrong interpolation kernel: {name}")
    success, encoded = cv2.imencode(".png", resized[:, :, ::-1], [cv2.IMWRITE_PNG_COMPRESSION, 9])
    if not success:
        raise ValueError(f"PNG encoding failed: {name}")
    decoded = cv2.imdecode(encoded, cv2.IMREAD_COLOR)[:, :, ::-1]
    if not np.array_equal(decoded, resized):
        raise ValueError(f"PNG round-trip changed RGB pixels: {name}")
    png = encoded.tobytes()
    (output / f"{name}.png").write_bytes(png)
    height, width = source.shape[:2]
    return {
        "name": name,
        "width": width,
        "height": height,
        "clockwise_quarter_turns": int(rotate),
        "input_rgb_sha256": sha256(source.tobytes()),
        "output_rgb_sha256": sha256(resized.tobytes()),
        "png_sha256": sha256(png),
        "im_shape": [float(INPUT_SIZE), float(INPUT_SIZE)],
        "scale_factor": [float(np.float32(INPUT_SIZE / height)), float(np.float32(INPUT_SIZE / width))],
        "linear_negative_control_max_error_u8": linear_error,
    }


def write_manifest(output: Path, manifest: dict) -> None:
    path = output / "manifest.json"
    path.write_text(json.dumps(manifest, indent=2) + "\n")
    formatter = shutil.which("poly")
    if formatter is None:
        raise RuntimeError("poly is required; run this generator through mise exec")
    formatter = str(Path(formatter).resolve())
    formatted = subprocess.run([formatter, "fmt", "--fix", "--include-excluded", str(path)], check=False)
    if formatted.returncode not in (0, 1):
        formatted.check_returncode()
    subprocess.run([formatter, "fmt", "--check", "--include-excluded", str(path)], check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-upstream", action="store_true")
    args = parser.parse_args()
    if cv2.__version__ != "4.10.0" or np.__version__ != "2.2.6":
        raise ValueError("Use the pinned OpenCV 4.10.0.84 and NumPy 2.2.6 dependencies")
    records = source_records()
    if args.verify_upstream:
        verify_upstream(records)
    cv2.setUseOptimized(False)
    cv2.setNumThreads(1)
    cv2.ocl.setUseOpenCL(False)
    output = Path(__file__).parent
    cases = [generate_case(case, output) for case in CASES]
    manifest = {
        "input_size": INPUT_SIZE,
        "generator_sha256": sha256(Path(__file__).read_bytes()),
        "max_error_u8": 1,
        "comparison": "All RGB values, including borders; one final u8 rounding and clamp before /255 NCHW",
        "tolerance_basis": "One u8 level permits OpenCV's 11-bit fixed-point coefficient rounding versus f32",
        "reference": {
            "python": platform.python_version(),
            "numpy": np.__version__,
            "opencv_python_headless": importlib.metadata.version("opencv-python-headless"),
            "optimized": cv2.useOptimized(),
            "threads": cv2.getNumThreads(),
            "opencl": cv2.ocl.useOpenCL(),
        },
        "sources": records,
        "cases": cases,
    }
    write_manifest(output, manifest)
    print(f"Generated {len(cases)} synthetic RGB references; {len(cases) * INPUT_SIZE * INPUT_SIZE * 3} values")


if __name__ == "__main__":
    main()
