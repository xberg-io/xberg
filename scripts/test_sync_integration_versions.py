import json

from scripts.sync_integration_versions import LAUNCHER_PIN_RE, helm_metadata_errors, mcp_metadata_errors, transform


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


def server_json(
    *, version: str = "1.5.0", package_version: str = "1.5.0", identifier: str = "@xberg-io/xberg-cli"
) -> str:
    return json.dumps(
        {
            "name": "io.github.xberg-io/xberg",
            "version": version,
            "packages": [{"registryType": "npm", "identifier": identifier, "version": package_version}],
        }
    )


def proxy_json(*, mcp_name: str | None = "io.github.xberg-io/xberg") -> str:
    data = {"name": "@xberg-io/xberg-cli", "version": "1.5.0"}
    if mcp_name is not None:
        data["mcpName"] = mcp_name
    return json.dumps(data)


def test_mcp_metadata_accepts_consistent_entry() -> None:
    assert mcp_metadata_errors(server_json(), proxy_json()) == []


def test_mcp_metadata_rejects_package_version_drift() -> None:
    assert mcp_metadata_errors(server_json(package_version="1.4.0"), proxy_json()) == [
        "server.json package version 1.4.0 != 1.5.0"
    ]


def test_mcp_metadata_rejects_missing_or_wrong_mcp_name() -> None:
    expected = ["cli-proxy/npm/package.json mcpName is {}, expected io.github.xberg-io/xberg"]
    assert mcp_metadata_errors(server_json(), proxy_json(mcp_name=None)) == [expected[0].format("None")]
    assert mcp_metadata_errors(server_json(), proxy_json(mcp_name="io.github.other/x")) == [
        expected[0].format("io.github.other/x")
    ]


def test_mcp_metadata_rejects_other_package_identifier() -> None:
    assert mcp_metadata_errors(server_json(identifier="xberg"), proxy_json()) == [
        "server.json must list exactly the npm package @xberg-io/xberg-cli"
    ]


def test_server_json_transform_rewrites_version_and_package_version_only() -> None:
    original, updated = transform("server.json", True, False, "1.6.0-rc.1", "1.6.0rc1")
    assert original != updated
    data = json.loads(updated)
    assert data["version"] == "1.6.0-rc.1"
    assert data["packages"][0]["version"] == "1.6.0-rc.1"
    assert data["$schema"] == json.loads(original)["$schema"]


def test_launcher_pin_rewrites_only_the_pinned_version_line() -> None:
    launcher = 'LAUNCHER="${XBERG_LAUNCHER:-auto}"\nPINNED_VERSION="1.4.2"\n'
    assert LAUNCHER_PIN_RE.sub(r"\g<1>1.5.0\g<3>", launcher, count=1) == (
        'LAUNCHER="${XBERG_LAUNCHER:-auto}"\nPINNED_VERSION="1.5.0"\n'
    )
