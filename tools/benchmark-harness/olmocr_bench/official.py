"""Use the original olmOCR evaluator with offline rendering and explicit counts."""

import importlib
import os
import socket
import sys
from collections import defaultdict
from collections.abc import Callable, Iterator
from concurrent.futures import ThreadPoolExecutor, as_completed
from contextlib import ExitStack, contextmanager
from functools import partial
from pathlib import Path
from typing import Any, NoReturn

from .contracts import GROUP_COUNTS, TEST_COUNT


def _deny_network(*_args: object, **_kwargs: object) -> NoReturn:
    raise RuntimeError("network forbidden during official local scoring")


def _restore_settings(previous: dict, previous_path: list[str]) -> None:
    sys.path[:] = previous_path
    for key, value in previous.items():
        if value is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = value


@contextmanager
def evaluation_environment(evaluator: Path, browsers: Path, home: Path) -> Iterator[None]:
    """Isolate import side effects and use the official renderer's cache-off option."""
    home.mkdir(parents=True, exist_ok=True)
    previous = {key: os.environ.get(key) for key in ("HOME", "PLAYWRIGHT_BROWSERS_PATH")}
    with ExitStack() as cleanup:
        cleanup.callback(_restore_settings, previous, list(sys.path))
        os.environ.update(HOME=str(home), PLAYWRIGHT_BROWSERS_PATH=str(browsers))
        sys.path.insert(0, str(evaluator))
        tests = importlib.import_module("olmocr.bench.tests")
        playwright = importlib.import_module("playwright.sync_api")
        if not Path(tests.__file__).resolve().is_relative_to(evaluator.resolve()):
            raise ValueError("imported evaluator does not match pinned checkout")
        original_context = playwright.Browser.new_context

        def offline_context(browser: Any, *args: Any, **kwargs: Any) -> Any:
            kwargs["offline"] = True
            return original_context(browser, *args, **kwargs)

        cleanup.callback(setattr, playwright.Browser, "new_context", original_context)
        cleanup.callback(setattr, tests, "render_equation", tests.render_equation)
        cleanup.callback(setattr, socket.socket, "connect", socket.socket.connect)
        cleanup.callback(setattr, socket.socket, "connect_ex", socket.socket.connect_ex)
        playwright.Browser.new_context = offline_context
        tests.render_equation = partial(tests.render_equation, use_cache=False)
        socket.socket.connect = _deny_network
        socket.socket.connect_ex = _deny_network
        yield


def load_official(definitions: list[dict]) -> tuple[list, list[dict]]:
    """Attempt every official constructor and retain initialization failures."""
    factory = importlib.import_module("olmocr.bench.tests").load_single_test
    tests, rows = [], []
    with ThreadPoolExecutor(max_workers=min(os.cpu_count() or 1, 64)) as executor:
        futures = {
            executor.submit(factory, {key: value for key, value in row.items() if key != "group"}): row["id"]
            for row in definitions
        }
        for future in as_completed(futures):
            identifier = futures[future]
            try:
                test = future.result()
                if test.type == "math" and (test.reference_render is None or test.reference_render.error):
                    error = test.reference_render.error if test.reference_render is not None else "missing render"
                    raise ValueError(f"invalid math reference render: {error}")
                tests.append(test)
            except Exception as error:
                rows.append({"id": identifier, "status": "initialization_error", "error": str(error)})
            else:
                rows.append({"id": identifier, "status": "initialized"})
    if len(rows) != TEST_COUNT or len({row["id"] for row in rows}) != TEST_COUNT:
        raise ValueError("official initialization did not attempt 6090 unique tests")
    return tests, rows


def score_group(candidate: Path, tests: list) -> dict:
    """Use official diagnostics mode but never treat candidate errors as qualification."""
    evaluator = importlib.import_module("olmocr.bench.benchmark").evaluate_candidate
    result = evaluator(str(candidate), tests, sorted({test.pdf for test in tests}), force=True)
    score, count, errors, failures, breakdown, scores, details = result
    rows = [
        {"id": test.id, "pdf": test.pdf, "page": page, "passed": passed, "explanation": explanation}
        for pages in details.values()
        for page, entries in pages.items()
        for test, passed, explanation in entries
    ]
    if (
        not tests
        or count != len(tests)
        or len(scores) != len(tests)
        or len(rows) != len(tests)
        or {row["id"] for row in rows} != {test.id for test in tests}
    ):
        raise ValueError("official evaluator coverage mismatch")
    return {
        "score": score,
        "tests": count,
        "errors": errors,
        "failures": failures,
        "test_type_scores": breakdown,
        "rows": rows,
    }


def score_candidate(candidate: Path, definitions: list[dict], tests: list, verify: Callable[[], None]) -> dict:
    """Preserve original JSONL groups, including seven explicit header baselines."""
    groups = {row["id"]: row["group"] for row in definitions}
    selected = defaultdict(list)
    for test in tests:
        selected[groups[test.id]].append(test)
    summaries, rows = {}, []
    for group, expected in GROUP_COUNTS.items():
        verify()
        result = score_group(candidate, selected[group])
        verify()
        if result["tests"] != expected:
            raise ValueError(f"official group count mismatch: {group}")
        rows.extend(dict(row, group=group) for row in result.pop("rows"))
        summaries[group] = result
    if len(rows) != TEST_COUNT or len({row["id"] for row in rows}) != TEST_COUNT:
        raise ValueError("missing or duplicate official test results")
    return {
        "groups": summaries,
        "tests": rows,
        "macro_score": sum(row["score"] for row in summaries.values()) / len(summaries),
        "errors": sum(len(row["errors"]) for row in summaries.values()),
    }
