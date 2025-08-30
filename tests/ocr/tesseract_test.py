from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import ANY, Mock, mock_open, patch

import pytest
from PIL import Image

from kreuzberg import PSMMode
from kreuzberg._ocr._tesseract import (
    TesseractBackend,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.fixture
def backend() -> TesseractBackend:
    return TesseractBackend()


@pytest.fixture
def mock_run_process(mocker: MockerFixture) -> Mock:
    async def async_run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 5.0.0"
        result.returncode = 0
        result.stderr = b""

        if "--version" in command and command[0].endswith("tesseract"):
            return result

        if len(command) >= 3 and command[0].endswith("tesseract"):
            output_file = command[2]
            if "test_process_image_with_tesseract_invalid_input" in str(kwargs.get("cwd")):
                result.returncode = 1
                result.stderr = b"Error processing file"
                raise OCRError("Error processing file")

            Path(f"{output_file}.txt").write_text("Sample OCR text")
            result.returncode = 0
            return result

        return result

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.return_value = Mock()
    mock.return_value.stdout = b"tesseract 5.0.0"
    mock.return_value.returncode = 0
    mock.return_value.stderr = b""
    mock.side_effect = async_run_sync
    return mock


@pytest.fixture
def mock_run_process_invalid(mocker: MockerFixture) -> Mock:
    async def run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 4.0.0"
        result.returncode = 0
        result.stderr = b""
        return result

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.return_value = Mock()
    mock.return_value.stdout = b"tesseract 4.0.0"
    mock.return_value.returncode = 0
    mock.side_effect = run_sync
    return mock


@pytest.fixture
def mock_run_process_error(mocker: MockerFixture) -> Mock:
    async def run_sync(command: list[str], **kwargs: Any) -> Mock:
        raise FileNotFoundError

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.side_effect = run_sync
    return mock


@pytest.mark.anyio
async def test_validate_tesseract_version(backend: TesseractBackend, mock_run_process: Mock) -> None:
    await backend._validate_tesseract_version()
    mock_run_process.assert_called_with(["tesseract", "--version"])


@pytest.fixture(autouse=True)
def reset_version_ref(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._ocr._tesseract.TesseractBackend._version_checked", False)


@pytest.mark.anyio
async def test_validate_tesseract_version_invalid(
    backend: TesseractBackend, mock_run_process_invalid: Mock, reset_version_ref: None
) -> None:
    with pytest.raises(MissingDependencyError) as excinfo:
        await backend._validate_tesseract_version()

    error_message = str(excinfo.value)
    assert "Tesseract version 5" in error_message
    assert "required" in error_message


@pytest.mark.anyio
async def test_validate_tesseract_version_missing(
    backend: TesseractBackend, mock_run_process_error: Mock, reset_version_ref: None
) -> None:
    with pytest.raises(MissingDependencyError) as excinfo:
        await backend._validate_tesseract_version()

    error_message = str(excinfo.value)
    assert "Tesseract version 5" in error_message
    assert "required" in error_message


@pytest.mark.anyio
async def test_process_file(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_with_options(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_error(
    backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path, fresh_cache: None
) -> None:
    async def error_side_effect(*args: Any, **kwargs: Any) -> Mock:
        if args and isinstance(args[0], list) and "--version" in args[0]:
            result = Mock()
            result.returncode = 0

            stdout_mock = Mock()
            stdout_mock.decode = Mock(return_value="tesseract 5.0.0")
            result.stdout = stdout_mock
            result.stderr = b""
            return result

        result = Mock()
        result.returncode = 1
        result.stderr = b"Error processing file"
        return result

    TesseractBackend._version_checked = False
    mock_run_process.side_effect = error_side_effect

    with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
        await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_file_runtime_error(
    backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path, fresh_cache: None
) -> None:
    call_count = 0

    async def runtime_error_side_effect(*args: Any, **kwargs: Any) -> Mock:
        nonlocal call_count
        call_count += 1

        if call_count == 1:
            result = Mock()
            result.returncode = 0

            stdout_mock = Mock()
            stdout_mock.decode = Mock(return_value="tesseract 5.0.0")
            result.stdout = stdout_mock
            result.stderr = b""
            return result

        raise RuntimeError("Command failed")

    TesseractBackend._version_checked = False
    mock_run_process.side_effect = runtime_error_side_effect

    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_image(backend: TesseractBackend, mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_image_with_tesseract_pillow(backend: TesseractBackend, mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await backend.process_image(image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_integration_process_file(backend: TesseractBackend, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_process_file_with_invalid_language(
    backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path
) -> None:
    with pytest.raises(ValidationError, match="not supported by Tesseract"):
        await backend.process_file(ocr_image, language="invalid", psm=PSMMode.AUTO)


@pytest.mark.parametrize(
    "language_code,expected_result",
    [
        ("eng", "eng"),
        ("ENG", "eng"),
        ("deu", "deu"),
        ("fra", "fra"),
        ("spa", "spa"),
        ("jpn", "jpn"),
        ("chi_sim", "chi_sim"),
        ("chi_tra", "chi_tra"),
    ],
)
def test_validate_language_code_valid(language_code: str, expected_result: str) -> None:
    result = TesseractBackend._validate_language_code(language_code)
    assert result == expected_result


@pytest.mark.parametrize(
    "invalid_language_code",
    [
        "invalid",
        "english",
        "español",
        "русский",
        "en",
        "de",
        "fr",
        "zh",
        "",
        "123",
    ],
)
def test_validate_language_code_invalid(invalid_language_code: str) -> None:
    with pytest.raises(ValidationError) as excinfo:
        TesseractBackend._validate_language_code(invalid_language_code)

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == invalid_language_code
    assert "supported_languages" in excinfo.value.context

    assert "not supported by Tesseract" in str(excinfo.value)


@pytest.mark.anyio
async def test_integration_process_image(backend: TesseractBackend, ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


@pytest.mark.anyio
async def test_process_file_linux(backend: TesseractBackend, mocker: MockerFixture, fresh_cache: None) -> None:
    mocker.patch("sys.platform", "linux")

    async def linux_mock_run(*args: Any, **kwargs: Any) -> Mock:
        result = Mock()
        result.returncode = 0
        result.stdout = b"tesseract 5.0.0" if "--version" in args[0] else b"test output"
        result.stderr = b""
        return result

    mock_run = mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=linux_mock_run)

    TesseractBackend._version_checked = False
    await backend.process_file(Path("test.png"), language="eng", psm=PSMMode.AUTO)

    assert any(call[1].get("env") == {"OMP_THREAD_LIMIT": "1"} for call in mock_run.call_args_list)


@pytest.mark.anyio
async def test_process_image_cache_processing_coordination(
    backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
) -> None:
    """Test cache processing coordination for process_image - covers lines 256-264."""
    from kreuzberg._utils._cache import get_ocr_cache

    test_image = Image.new("RGB", (100, 50), color="white")

    mocker.patch(
        "kreuzberg._ocr._tesseract.run_process", return_value=Mock(returncode=0, stdout=b"tesseract 5.0.0", stderr=b"")
    )

    import anyio

    cache = get_ocr_cache()

    import hashlib

    image_bytes = b"fake image bytes"
    image_hash = hashlib.sha256(image_bytes).hexdigest()[:16]

    cache.mark_processing(image_hash=image_hash, config="test_config")

    async def complete_processing(event: anyio.Event) -> None:
        await anyio.sleep(0.1)
        cache.mark_complete(image_hash=image_hash, config="test_config")

        cache.set(
            ExtractionResult(content="cached text", mime_type="text/plain", metadata={}, chunks=[], tables=[]),
            image_hash=image_hash,
            config="test_config",
        )
        event.set()

    async with anyio.create_task_group() as nursery:
        completion_event = anyio.Event()
        nursery.start_soon(complete_processing, completion_event)

        mock_hash_obj = Mock()
        mock_hash_obj.hexdigest.return_value = image_hash + "0" * 48
        mocker.patch("kreuzberg._ocr._tesseract.hashlib.sha256", return_value=mock_hash_obj)

        result = await backend.process_image(test_image, language="eng")

        assert result.content == "cached text"

        await completion_event.wait()


@pytest.mark.anyio
async def test_process_file_cache_processing_coordination(
    backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
) -> None:
    """Test cache processing coordination for process_file - covers lines 323-331."""
    from kreuzberg._utils._cache import get_ocr_cache

    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    mocker.patch(
        "kreuzberg._ocr._tesseract.run_process", return_value=Mock(returncode=0, stdout=b"tesseract 5.0.0", stderr=b"")
    )

    import anyio

    cache = get_ocr_cache()

    # Generate cache key based on file - must match the format in process_file  # ~keep
    file_stat = test_file.stat()
    file_info = {
        "path": str(test_file.resolve()),
        "size": file_stat.st_size,
        "mtime": file_stat.st_mtime,
    }
    cache_kwargs = {
        "file_info": str(sorted(file_info.items())),
        "ocr_backend": "tesseract",
        "ocr_config": str(sorted([("language", "eng")])),
    }

    cache.mark_processing(**cache_kwargs)

    async def complete_processing(event: anyio.Event) -> None:
        await anyio.sleep(0.1)
        cache.mark_complete(**cache_kwargs)
        cache.set(
            ExtractionResult(content="cached file text", mime_type="text/plain", metadata={}, chunks=[], tables=[]),
            **cache_kwargs,
        )
        event.set()

    async with anyio.create_task_group() as nursery:
        completion_event = anyio.Event()
        nursery.start_soon(complete_processing, completion_event)

        # This should trigger cache coordination  # ~keep
        result = await backend.process_file(test_file, language="eng")

        # Should get cached result  # ~keep
        assert result.content == "cached file text"

        await completion_event.wait()


def test_validate_language_code_error() -> None:
    """Test language code validation error - covers line 436."""
    backend = TesseractBackend()

    with pytest.raises(ValidationError, match="provided language code is not supported"):
        backend._validate_language_code("invalid_language_code_that_is_too_long_and_invalid")


@pytest.mark.anyio
async def test_process_image_validation_error(backend: TesseractBackend) -> None:
    """Test validation error in process_image - covers line 251."""

    test_image = Image.new("RGB", (1, 1), color="white")

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_image(test_image, language="invalid")


@pytest.mark.anyio
async def test_process_file_validation_error(backend: TesseractBackend, tmp_path: Path) -> None:
    """Test validation error in process_file - covers line 357."""

    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_file(test_file, language="invalid")


def test_process_image_sync(backend: TesseractBackend) -> None:
    """Test sync image processing."""

    image = Image.new("RGB", (100, 100))

    with (
        patch.object(backend, "_run_tesseract_sync") as mock_run,
        patch("tempfile.NamedTemporaryFile") as mock_temp,
        patch("pathlib.Path.open", mock_open(read_data="Sample OCR text")),
        patch.object(backend, "_validate_tesseract_version_sync"),
    ):
        mock_run.return_value = None
        mock_temp_file = Mock()
        mock_temp_file.name = "test_image"
        mock_temp.return_value.__enter__.return_value = mock_temp_file

        result = backend.process_image_sync(image, language="eng")

        assert isinstance(result, ExtractionResult)
        assert result.content.strip() == "Sample OCR text"


def test_process_file_sync(backend: TesseractBackend, ocr_image: Path) -> None:
    """Test sync file processing."""

    with (
        patch.object(backend, "_run_tesseract_sync") as mock_run,
        patch("tempfile.NamedTemporaryFile") as mock_temp,
        patch("pathlib.Path.open", mock_open(read_data="Sample file text")),
        patch.object(backend, "_validate_tesseract_version_sync"),
    ):
        mock_run.return_value = None
        mock_temp_file = Mock()
        mock_temp_file.name = "test_output"
        mock_temp.return_value.__enter__.return_value = mock_temp_file

        result = backend.process_file_sync(ocr_image, language="eng")

        assert isinstance(result, ExtractionResult)
        assert result.content.strip() == "Sample file text"


class TestTesseractConfigValidation:
    """Test comprehensive TesseractConfig validation."""

    def test_tesseract_config_all_parameters(self) -> None:
        """Test TesseractConfig with all parameters."""
        from kreuzberg._ocr._tesseract import TesseractConfig

        config = TesseractConfig(
            language="eng+deu",
            psm=PSMMode.SINGLE_BLOCK,
            tessedit_char_whitelist="0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            tessedit_enable_dict_correction=False,
            language_model_ngram_on=True,
            textord_space_size_is_variable=False,
            tessedit_dont_blkrej_good_wds=True,
            tessedit_dont_rowrej_good_wds=False,
            tessedit_use_primary_params_model=True,
            classify_use_pre_adapted_templates=False,
            thresholding_method=True,
        )

        assert config.language == "eng+deu"
        assert config.psm == PSMMode.SINGLE_BLOCK
        assert "0123456789" in config.tessedit_char_whitelist
        assert config.tessedit_enable_dict_correction is False
        assert config.language_model_ngram_on is True
        assert config.textord_space_size_is_variable is False
        assert config.tessedit_dont_blkrej_good_wds is True
        assert config.tessedit_dont_rowrej_good_wds is False
        assert config.tessedit_use_primary_params_model is True
        assert config.classify_use_pre_adapted_templates is False
        assert config.thresholding_method is True

    def test_tesseract_config_default_values(self) -> None:
        """Test TesseractConfig with default values."""
        from kreuzberg._ocr._tesseract import TesseractConfig

        config = TesseractConfig()

        assert config.language == "eng"
        assert config.psm == PSMMode.AUTO
        assert config.tessedit_char_whitelist == ""
        assert config.tessedit_enable_dict_correction is True
        assert config.language_model_ngram_on is False
        assert config.textord_space_size_is_variable is True
        assert config.tessedit_dont_blkrej_good_wds is True
        assert config.tessedit_dont_rowrej_good_wds is True
        assert config.tessedit_use_primary_params_model is True
        assert config.classify_use_pre_adapted_templates is True
        assert config.thresholding_method is False

    @pytest.mark.parametrize(
        "psm_mode",
        [
            PSMMode.OSD_ONLY,
            PSMMode.AUTO_OSD,
            PSMMode.AUTO_ONLY,
            PSMMode.AUTO,
            PSMMode.SINGLE_COLUMN,
            PSMMode.SINGLE_BLOCK_VERTICAL,
            PSMMode.SINGLE_BLOCK,
            PSMMode.SINGLE_LINE,
            PSMMode.SINGLE_WORD,
            PSMMode.CIRCLE_WORD,
            PSMMode.SINGLE_CHAR,
        ],
    )
    def test_psm_mode_values(self, psm_mode: PSMMode) -> None:
        """Test all PSM mode values."""
        from kreuzberg._ocr._tesseract import TesseractConfig

        config = TesseractConfig(psm=psm_mode)
        assert config.psm == psm_mode
        assert isinstance(psm_mode.value, int)
        assert 0 <= psm_mode.value <= 10


class TestTesseractCommandBuilding:
    """Test tesseract command building functionality."""

    def test_build_tesseract_command_basic(self, backend: TesseractBackend) -> None:
        """Test basic tesseract command building."""
        command = backend._build_tesseract_command(
            path=Path("input.png"), output_base="output", language="eng", psm=PSMMode.AUTO
        )

        assert command[0] == "tesseract"
        assert "input.png" in command
        assert "output" in command
        assert "-l" in command
        assert "eng" in command
        assert "--psm" in command
        assert "3" in command

    def test_build_tesseract_command_complex(self, backend: TesseractBackend) -> None:
        """Test complex tesseract command building with all options."""
        command = backend._build_tesseract_command(
            path=Path("complex_input.tiff"),
            output_base="complex_output",
            language="eng+deu+fra",
            psm=PSMMode.SINGLE_BLOCK,
            tessedit_char_whitelist="0123456789",
            tessedit_enable_dict_correction=False,
            language_model_ngram_on=False,
            textord_space_size_is_variable=True,
            tessedit_dont_blkrej_good_wds=False,
        )

        assert "tesseract" in command[0]
        assert "complex_input.tiff" in command
        assert "complex_output" in command
        assert "-l" in command
        assert "eng+deu+fra" in command
        assert "--psm" in command
        assert "6" in command

        command_str = " ".join(command)
        assert "tessedit_char_whitelist=0123456789" in command_str
        assert "tessedit_enable_dict_correction=0" in command_str
        assert "language_model_ngram_on=0" in command_str
        assert "textord_space_size_is_variable=1" in command_str
        assert "tessedit_dont_blkrej_good_wds=0" in command_str
        assert "tessedit_dont_rowrej_good_wds=1" in command_str
        assert "classify_use_pre_adapted_templates=1" in command_str
        assert "thresholding_method=0" in command_str

    def test_build_tesseract_command_no_config(self, backend: TesseractBackend) -> None:
        """Test tesseract command building with no config."""
        command = backend._build_tesseract_command(
            path=Path("input.jpg"), output_base="output", language="eng", psm=PSMMode.AUTO
        )

        assert command[0] == "tesseract"
        assert "input.jpg" in command
        assert "output" in command
        assert "-l" in command
        assert "eng" in command


class TestTesseractFileHandling:
    """Test file handling functionality."""

    def test_get_file_info(self, backend: TesseractBackend, tmp_path: Path) -> None:
        """Test file info extraction."""
        test_file = tmp_path / "test_file.png"
        test_file.write_text("dummy content")

        file_info = backend._get_file_info(test_file)

        assert "path" in file_info
        assert "size" in file_info
        assert "mtime" in file_info
        assert file_info["path"] == str(test_file.resolve())
        assert file_info["size"] == len("dummy content")
        assert isinstance(file_info["mtime"], float)

    def test_get_file_info_nonexistent(self, backend: TesseractBackend) -> None:
        """Test file info with nonexistent file."""
        nonexistent = Path("/nonexistent/file.png")

        with pytest.raises(FileNotFoundError):
            backend._get_file_info(nonexistent)


class TestTesseractLanguageValidation:
    """Test comprehensive language validation."""

    @pytest.mark.parametrize(
        "language_code",
        [
            "afr",
            "amh",
            "ara",
            "asm",
            "aze",
            "aze_cyrl",
            "bel",
            "ben",
            "bod",
            "bos",
            "bre",
            "bul",
            "cat",
            "ceb",
            "ces",
            "chi_sim",
            "chi_tra",
            "chr",
            "cos",
            "cym",
            "dan",
            "deu",
            "div",
            "dzo",
            "ell",
            "eng",
            "enm",
            "epo",
            "est",
            "eus",
            "fao",
            "fas",
            "fil",
            "fin",
            "fra",
            "frk",
            "frm",
            "fry",
            "gla",
            "gle",
            "glg",
            "grc",
            "guj",
            "hat",
            "heb",
            "hin",
            "hrv",
            "hun",
            "hye",
            "iku",
            "ind",
            "isl",
            "ita",
            "ita_old",
            "jav",
            "jpn",
            "kan",
            "kat",
            "kat_old",
            "kaz",
            "khm",
            "kir",
            "kor",
            "kur",
            "lao",
            "lat",
            "lav",
            "lit",
            "ltz",
            "mal",
            "mar",
            "mkd",
            "mlt",
            "mon",
            "mri",
            "msa",
            "mya",
            "nep",
            "nld",
            "nor",
            "oci",
            "ori",
            "pan",
            "pol",
            "por",
            "pus",
            "que",
            "ron",
            "rus",
            "san",
            "sin",
            "slk",
            "slv",
            "snd",
            "spa",
            "spa_old",
            "sqi",
            "srp",
            "srp_latn",
            "sun",
            "swa",
            "swe",
            "syr",
            "tam",
            "tat",
            "tel",
            "tgk",
            "tha",
            "tir",
            "ton",
            "tur",
            "uig",
            "ukr",
            "urd",
            "uzb",
            "uzb_cyrl",
            "vie",
            "yid",
            "yor",
        ],
    )
    def test_all_supported_language_codes(self, language_code: str) -> None:
        """Test all supported language codes."""
        result = TesseractBackend._validate_language_code(language_code)
        assert result == language_code.lower()

    def test_multi_language_codes(self) -> None:
        """Test multiple language codes."""
        result = TesseractBackend._validate_language_code("eng+deu+fra")
        assert result == "eng+deu+fra"

        result = TesseractBackend._validate_language_code("chi_sim+eng")
        assert result == "chi_sim+eng"

    def test_case_insensitive_language_codes(self) -> None:
        """Test case insensitive language codes."""
        result = TesseractBackend._validate_language_code("ENG")
        assert result == "eng"

        result = TesseractBackend._validate_language_code("DEU+FRA")
        assert result == "deu+fra"


class TestTesseractSyncMethods:
    """Test synchronous methods."""

    def test_run_tesseract_sync_success(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test successful sync tesseract execution."""
        mock_run = mocker.patch("subprocess.run")
        mock_result = Mock()
        mock_result.returncode = 0
        mock_result.stderr = ""
        mock_run.return_value = mock_result

        command = ["tesseract", "input.png", "output", "-l", "eng"]
        backend._run_tesseract_sync(command)

        mock_run.assert_called_once_with(command, check=False, env=ANY, capture_output=True, text=True, timeout=30)

    def test_run_tesseract_sync_error(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test sync tesseract execution with error."""
        mock_run = mocker.patch("subprocess.run")
        mock_result = Mock()
        mock_result.returncode = 1
        mock_result.stderr = "Tesseract error occurred"
        mock_run.return_value = mock_result

        command = ["tesseract", "input.png", "output", "-l", "eng"]

        with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
            backend._run_tesseract_sync(command)

    def test_run_tesseract_sync_runtime_error(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test sync tesseract execution with runtime error."""
        mock_run = mocker.patch("subprocess.run")
        mock_run.side_effect = RuntimeError("Command execution failed")

        command = ["tesseract", "input.png", "output", "-l", "eng"]

        with pytest.raises(RuntimeError, match="Command execution failed"):
            backend._run_tesseract_sync(command)

    def test_validate_tesseract_version_sync_success(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test successful sync version validation."""
        mock_run = mocker.patch("subprocess.run")
        mock_result = Mock()
        mock_result.returncode = 0
        mock_result.stdout = "tesseract 5.2.0"
        mock_result.stderr = ""
        mock_run.return_value = mock_result

        TesseractBackend._version_checked = False

        backend._validate_tesseract_version_sync()

        mock_run.assert_called_once_with(["tesseract", "--version"], capture_output=True, text=True, check=False)
        assert TesseractBackend._version_checked is True

    def test_validate_tesseract_version_sync_too_old(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test sync version validation with old version."""
        mock_run = mocker.patch("subprocess.run")
        mock_result = Mock()
        mock_result.returncode = 0
        mock_result.stdout = "tesseract 4.1.1"
        mock_result.stderr = ""
        mock_run.return_value = mock_result

        TesseractBackend._version_checked = False

        with pytest.raises(MissingDependencyError, match="Tesseract version 5"):
            backend._validate_tesseract_version_sync()

    def test_validate_tesseract_version_sync_not_found(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test sync version validation when tesseract not found."""
        mock_run = mocker.patch("subprocess.run")
        mock_run.side_effect = FileNotFoundError("tesseract not found")

        TesseractBackend._version_checked = False

        with pytest.raises(MissingDependencyError, match="Tesseract version 5"):
            backend._validate_tesseract_version_sync()


class TestTesseractEnvironmentVariables:
    """Test environment variable handling."""

    @pytest.mark.anyio
    async def test_linux_omp_thread_limit(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test OMP_THREAD_LIMIT environment variable on Linux."""
        mocker.patch("sys.platform", "linux")

        async def mock_run_process(*args: Any, **kwargs: Any) -> Mock:
            assert kwargs.get("env") == {"OMP_THREAD_LIMIT": "1"}
            result = Mock()
            result.returncode = 0
            result.stdout = b"tesseract 5.0.0" if "--version" in args[0] else b""
            result.stderr = b""
            return result

        mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=mock_run_process)

        TesseractBackend._version_checked = False

        test_image = Image.new("RGB", (100, 100), "white")
        await backend.process_image(test_image, language="eng")

    @pytest.mark.anyio
    async def test_non_linux_no_env_vars(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test no environment variables on non-Linux platforms."""
        mocker.patch("sys.platform", "darwin")

        async def mock_run_process(*args: Any, **kwargs: Any) -> Mock:
            assert kwargs.get("env") is None
            result = Mock()
            result.returncode = 0
            result.stdout = b"tesseract 5.0.0" if "--version" in args[0] else b""
            result.stderr = b""
            return result

        mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=mock_run_process)

        TesseractBackend._version_checked = False

        test_image = Image.new("RGB", (100, 100), "white")
        await backend.process_image(test_image, language="eng")


class TestTesseractImageProcessing:
    """Test image processing functionality."""

    @pytest.mark.anyio
    async def test_process_image_with_different_modes(self, backend: TesseractBackend, mock_run_process: Mock) -> None:
        """Test image processing with different image modes."""
        modes = ["RGB", "RGBA", "L", "P", "CMYK"]

        for mode in modes:
            if mode == "CMYK":
                image = Image.new("RGB", (100, 100), "white").convert("CMYK")
            else:
                image = Image.new(mode, (100, 100), "white")

            result = await backend.process_image(image, language="eng")
            assert isinstance(result, ExtractionResult)
            assert result.content.strip() == "Sample OCR text"

    @pytest.mark.anyio
    async def test_process_image_very_small(self, backend: TesseractBackend, mock_run_process: Mock) -> None:
        """Test processing very small images."""
        image = Image.new("RGB", (1, 1), "white")
        result = await backend.process_image(image, language="eng")
        assert isinstance(result, ExtractionResult)

    @pytest.mark.anyio
    async def test_process_image_very_large(self, backend: TesseractBackend, mock_run_process: Mock) -> None:
        """Test processing large images."""
        image = Image.new("RGB", (2000, 1500), "white")
        result = await backend.process_image(image, language="eng")
        assert isinstance(result, ExtractionResult)


class TestTesseractErrorHandling:
    """Test comprehensive error handling."""

    @pytest.mark.anyio
    async def test_process_file_file_not_found(self, backend: TesseractBackend) -> None:
        """Test processing non-existent file."""
        nonexistent_file = Path("/nonexistent/file.png")

        with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
            await backend.process_file(nonexistent_file, language="eng")

    @pytest.mark.anyio
    async def test_process_image_invalid_format(self, backend: TesseractBackend, mock_run_process: Mock) -> None:
        """Test processing corrupted/invalid image."""
        image = Image.new("RGB", (100, 100), "white")

        async def error_side_effect(*args: Any, **kwargs: Any) -> Mock:
            if "--version" in args[0]:
                result = Mock()
                result.returncode = 0
                result.stdout = b"tesseract 5.0.0"
                result.stderr = b""
                return result

            result = Mock()
            result.returncode = 1
            result.stderr = b"Error: Image format not supported"
            return result

        mock_run_process.side_effect = error_side_effect

        with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
            await backend.process_image(image, language="eng")

    def test_sync_process_image_temp_file_error(self, backend: TesseractBackend, mocker: MockerFixture) -> None:
        """Test sync image processing with temp file creation error."""
        image = Image.new("RGB", (100, 100), "white")

        mocker.patch("tempfile.NamedTemporaryFile", side_effect=OSError("Cannot create temp file"))

        with pytest.raises(OSError, match="Cannot create temp file"):
            backend.process_image_sync(image, language="eng")

    def test_sync_process_file_read_error(
        self, backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test sync file processing with read error."""
        test_file = tmp_path / "test.png"
        test_file.write_bytes(b"fake image data")

        mocker.patch.object(backend, "_validate_tesseract_version_sync")
        mocker.patch.object(backend, "_run_tesseract_sync")
        mocker.patch("pathlib.Path.open", side_effect=OSError("Cannot read file"))

        with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
            backend.process_file_sync(test_file, language="eng")


class TestTesseractConfigEdgeCases:
    """Test configuration edge cases."""

    def test_empty_whitelist(self) -> None:
        """Test empty whitelist."""
        from kreuzberg._ocr._tesseract import TesseractConfig

        config = TesseractConfig(tessedit_char_whitelist="")
        assert config.tessedit_char_whitelist == ""

    def test_very_long_whitelist(self) -> None:
        """Test very long whitelist."""
        from kreuzberg._ocr._tesseract import TesseractConfig

        long_whitelist = "".join(chr(i) for i in range(32, 127))

        config = TesseractConfig(tessedit_char_whitelist=long_whitelist)
        assert len(config.tessedit_char_whitelist) > 90

    def test_unicode_language_combinations(self) -> None:
        """Test various Unicode language combinations."""
        valid_combinations = [
            "ara+eng",
            "chi_sim+eng+deu",
            "jpn+kor+eng",
            "rus+ukr+eng",
            "hin+pan+urd+eng",
        ]

        for combo in valid_combinations:
            result = TesseractBackend._validate_language_code(combo)
            assert result == combo.lower()


class TestTesseractUtilityFunctions:
    """Test utility functions."""

    def test_normalize_spaces_in_results(self, backend: TesseractBackend, mock_run_process: Mock) -> None:
        """Test that normalize_spaces is properly applied to results."""

        async def mock_with_extra_spaces(*args: Any, **kwargs: Any) -> Mock:
            if "--version" in args[0]:
                result = Mock()
                result.returncode = 0
                result.stdout = b"tesseract 5.0.0"
                result.stderr = b""
                return result

            output_file = args[0][2]
            Path(f"{output_file}.txt").write_text("This  has   extra    spaces\nAnd\t\ttabs\n\n\nAnd newlines")

            result = Mock()
            result.returncode = 0
            result.stderr = b""
            return result

        mock_run_process.side_effect = mock_with_extra_spaces

        image = Image.new("RGB", (100, 100), "white")

        with (
            patch.object(backend, "_validate_tesseract_version_sync"),
            patch("tempfile.NamedTemporaryFile") as mock_temp,
        ):
            mock_temp_file = Mock()
            mock_temp_file.name = "test_output"
            mock_temp.return_value.__enter__.return_value = mock_temp_file

            result = backend.process_image_sync(image, language="eng")

            assert "  " not in result.content
            assert "\t\t" not in result.content
            assert "\n\n\n" not in result.content


@pytest.mark.anyio
async def test_tesseract_concurrent_processing(backend: TesseractBackend, mock_run_process: Mock) -> None:
    """Test concurrent image processing."""
    import asyncio

    images = [Image.new("RGB", (50, 50), f"color{i}") for i in range(5)]

    async def process_image(img: Any) -> ExtractionResult:
        return await backend.process_image(img, language="eng")

    results = await asyncio.gather(*[process_image(img) for img in images])

    assert len(results) == 5
    for result in results:
        assert isinstance(result, ExtractionResult)
        assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_tesseract_memory_efficiency(backend: TesseractBackend, mock_run_process: Mock) -> None:
    """Test memory efficiency with large images."""
    large_image = Image.new("RGB", (1000, 1000), "white")

    result = await backend.process_image(large_image, language="eng")
    assert isinstance(result, ExtractionResult)

    import gc

    del large_image
    gc.collect()

    small_image = Image.new("RGB", (100, 100), "white")
    result2 = await backend.process_image(small_image, language="eng")
    assert isinstance(result2, ExtractionResult)
