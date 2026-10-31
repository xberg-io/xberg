"""Hash-bound inputs and source isolation for the prospective tap diagnostic."""

import importlib.util
import sys
from pathlib import Path
from types import ModuleType

from .assets import digest, read_json
from .first_divergence import ATOL, LAYERS, RTOL, STAGES

PROTOCOL = {
    "prefix_lengths": [1, 2],
    "image": "zeros",
    "layers": LAYERS,
    "hidden": 768,
    "vocabulary": 50257,
    "ort_optimization": "ORT_DISABLE_ALL",
    "ort_provider": "CPUExecutionProvider",
    "ort_execution": "sequential",
    "ort_threads": 1,
    "atol": ATOL,
    "rtol": RTOL,
    "expected_counts": {"cases": 2, "stages": 30, "compared_elements": 183027},
    "expected_controls": {"observer": 2, "mask": 2, "negative": 2},
}


def require(condition: bool, message: str) -> None:
    """Abort a diagnostic on missing evidence rather than imply qualification."""
    if not condition:
        raise ValueError(message)


def verify_runtime(plan: dict) -> None:
    """Bind the entire actual package and reject an imported foreign checkout."""
    root = Path(__file__).resolve().parent
    require(root == Path(plan["source_root"]).resolve(), "runtime source root differs from frozen checkout")
    paths = list(root.rglob("*.py"))
    actual = {str(path.relative_to(root)): digest(path) for path in paths}
    require(bool(actual) and actual == plan["runtime_sources"], "runtime Python source inventory changed")
    require(not any(path.is_symlink() for path in paths), "runtime Python source symlink is unsupported")
    for name, module in list(sys.modules.items()):
        if name == "caption_export" or name.startswith("caption_export."):
            location = getattr(module, "__file__", None)
            require(
                location is not None and Path(location).resolve().is_relative_to(root), "foreign caption module loaded"
            )


def verify_original(plan: dict) -> tuple[ModuleType, dict, dict]:
    """Replay the already frozen source/model/loading gates without rewriting them."""
    require(not sys.flags.optimize, "diagnostic requires original preflight assertions enabled")
    verify_runtime(plan)
    require(plan["version"] == 1 and plan["protocol"] == PROTOCOL, "unsupported diagnostic protocol")
    directory = Path(plan["original_directory"])
    require(set(plan["original_files"]) == {"plan.json", "parity.py", "result.json"}, "original pins incomplete")
    for name, checksum in plan["original_files"].items():
        path = directory / name
        require(not path.is_symlink() and digest(path) == checksum, f"original evidence checksum mismatch: {name}")
    previous = read_json(directory / "result.json")
    original_plan = read_json(directory / "plan.json")
    require(previous["status"] == "FAIL", "original qualification must remain failed")
    require(previous["actual_counts"] == original_plan["expected_counts"], "original qualification count mismatch")
    failed = sorted((row["image"], row["length"]) for row in previous["prefix"] if not row["passed"])
    require(
        failed == sorted((image, length) for image in ("zeros", "uniform") for length in (2, 8, 16)),
        "original six failed prefixes changed",
    )
    require((original_plan["atol"], original_plan["rtol"]) == (ATOL, RTOL), "original tolerances changed")
    require(
        original_plan["geometry"]
        == {"image": 224, "context": 197, "hidden": 768, "vocabulary": 50257, "layers": LAYERS},
        "unsupported original architecture",
    )
    spec = importlib.util.spec_from_file_location("frozen_caption_parity", directory / "parity.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.preflight(original_plan, directory)
    return module, original_plan, previous


def verify_output(output: Path, plan: dict, original_plan: dict, plan_path: Path) -> None:
    """Require a new, disjoint output directory, never a source/evidence overwrite."""
    require(not output.exists() and not output.is_symlink(), "diagnostic output already exists")
    protected = [Path(plan["original_directory"]), Path(plan["source_root"]), plan_path]
    protected.extend(Path(original_plan[key]) for key in ("source", "bundle"))
    for path in protected:
        require(not output.resolve().is_relative_to(path.resolve()), "output overlaps immutable inputs")
        require(not path.resolve().is_relative_to(output.resolve()), "output encloses immutable inputs")


def runtime_receipt(model: object) -> dict:
    """Record actual hook module and forward identities for all fifteen seams."""
    modules = hook_modules(model)
    return {
        stage: {
            "class": f"{type(module).__module__}.{type(module).__qualname__}",
            "forward": f"{module.forward.__module__}.{module.forward.__qualname__}",
        }
        for stage, module in modules.items()
    }


def hook_modules(model: object) -> dict:
    """Select the actual decoder module objects, without replacing their methods."""
    transformer = model.decoder.transformer
    require(len(transformer.h) == LAYERS, "decoder layer count mismatch")
    modules = {"embedding": transformer.drop}
    modules.update({f"block.{index}": block for index, block in enumerate(transformer.h)})
    modules.update(final_norm=transformer.ln_f, logits=model.decoder.lm_head)
    require(tuple(modules) == STAGES, "hook stage inventory mismatch")
    return modules
