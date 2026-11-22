from scripts.sync_integration_versions import LAUNCHER_PIN_RE, helm_metadata_errors


def chart_metadata(
    *,
    version: str,
    app_version: str | None = None,
    image_version: str | None = None,
    prerelease: str = "false",
) -> str:
    app_version = app_version or version
    image_version = image_version or version
    return f"""\
apiVersion: v2
name: xberg
version: {version}
appVersion: "{app_version}"
annotations:
  artifacthub.io/prerelease: "{prerelease}"
  artifacthub.io/images: |
    - name: xberg
      image: ghcr.io/xberg-io/xberg:{image_version}
"""


def test_helm_metadata_accepts_matching_stable_version() -> None:
    assert helm_metadata_errors(chart_metadata(version="1.1.0"), "1.1.0") == []


def test_helm_metadata_accepts_matching_prerelease_version() -> None:
    chart = chart_metadata(version="1.1.0-rc.2", prerelease="true")

    assert helm_metadata_errors(chart, "1.1.0-rc.2") == []


def test_helm_metadata_treats_build_metadata_as_stable() -> None:
    chart = chart_metadata(version="1.1.0+linux.1")

    assert helm_metadata_errors(chart, "1.1.0+linux.1") == []


def test_helm_metadata_reports_every_mismatched_release_field() -> None:
    chart = chart_metadata(
        version="1.0.14",
        app_version="1.0.13",
        image_version="1.0.0-rc.25",
        prerelease="true",
    )

    assert helm_metadata_errors(chart, "1.0.14") == [
        "chart appVersion is 1.0.13, expected 1.0.14",
        "Artifact Hub image tag is 1.0.0-rc.25, expected 1.0.14",
        "Artifact Hub prerelease is true, expected false",
    ]


def test_helm_metadata_rejects_missing_required_annotation() -> None:
    chart = chart_metadata(version="1.1.0").replace(
        '  artifacthub.io/prerelease: "false"\n',
        "",
    )

    assert helm_metadata_errors(chart, "1.1.0") == [
        "chart is missing artifacthub.io/prerelease",
    ]


def test_helm_metadata_rejects_invalid_expected_semver() -> None:
    assert helm_metadata_errors(chart_metadata(version="release"), "release") == [
        "release version is not valid SemVer: release",
    ]


def test_helm_metadata_rejects_leading_zero_prerelease_number() -> None:
    version = "1.1.0-rc.02"

    assert helm_metadata_errors(chart_metadata(version=version), version) == [
        f"release version is not valid SemVer: {version}",
    ]


def test_launcher_pin_rewrites_only_the_pinned_version_line() -> None:
    launcher = 'LAUNCHER="${XBERG_LAUNCHER:-auto}"\nPINNED_VERSION="1.4.2"\n'
    assert LAUNCHER_PIN_RE.sub(r"\g<1>1.5.0\g<3>", launcher, count=1) == (
        'LAUNCHER="${XBERG_LAUNCHER:-auto}"\nPINNED_VERSION="1.5.0"\n'
    )
