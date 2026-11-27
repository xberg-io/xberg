//! CBOM (Cryptographic Bill of Materials) export.
//!
//! Serialises [`super::inventory`] (the set of [`AlgorithmId`]s
//! actually exercised this process — the lock-free `AtomicU64` bitset)
//! into a **CycloneDX 1.6** `cryptographic-asset`
//! BOM. This is the machine-readable form of the "what crypto did this
//! run use" report regulated buyers ask for: the
//! complement to the human-readable [`super::inventory`] token list.
//!
//! Pure additive read-side governance — no new primitive, no policy
//! behaviour change. The document is built with `serde_json` (already
//! a dependency) so the shape is well-formed by construction.

use super::error::AlgorithmKind;
use super::policy::AlgorithmId;

/// The CycloneDX `cryptoProperties.algorithmProperties.primitive`
/// token for this algorithm (RC4 is the only stream cipher we expose;
/// AES-CBC is a block cipher; everything else maps from its
/// [`AlgorithmKind`]).
fn primitive(alg: AlgorithmId) -> &'static str {
    match alg {
        AlgorithmId::CipherRc4 => "stream-cipher",
        _ => match alg.kind() {
            AlgorithmKind::Hash => "hash",
            AlgorithmKind::SymmetricCipher => "block-cipher",
            AlgorithmKind::SignatureSign | AlgorithmKind::SignatureVerify => "signature",
            AlgorithmKind::KeyDerivation => "kdf",
            AlgorithmKind::RandomBytes => "drbg",
        },
    }
}

/// Build one CycloneDX `cryptographic-asset` component for `alg`.
fn component(alg: AlgorithmId) -> serde_json::Value {
    let token = alg.token();
    let mut cert = Vec::new();
    if alg.is_fips_approved() {
        cert.push("fips140-3");
    } else {
        cert.push("none");
    }
    serde_json::json!({
        "type": "cryptographic-asset",
        "name": token,
        "bom-ref": format!("crypto/algorithm/{token}"),
        "cryptoProperties": {
            "assetType": "algorithm",
            "algorithmProperties": {
                "primitive": primitive(alg),
                // `min_security_bits` is the policy floor this build
                // associates with the algorithm (0 for the legacy
                // primitives it never lets you *write* with). ~keep
                "classicalSecurityLevel": alg.min_security_bits(),
                "nistQuantumSecurityLevel": 0,
                "certificationLevel": cert,
            }
        }
    })
}

/// RFC 3339 UTC timestamp with a `+00:00` offset and a fractional-seconds part
/// only when non-zero (the shape `chrono::DateTime::to_rfc3339` produced).
fn format_rfc3339(ts: jiff::Timestamp) -> String {
    ts.strftime("%Y-%m-%dT%H:%M:%S%.f+00:00").to_string()
}

/// A CycloneDX 1.6 Cryptographic Bill of Materials (JSON string) of
/// every algorithm exercised so far this process.
///
/// Stable, well-formed JSON: `bomFormat`/`specVersion`, a `metadata`
/// block (RFC 3339 timestamp + the `xberg-native-pdf` tool component), and one
/// `cryptographic-asset` component per exercised [`AlgorithmId`]. An
/// empty inventory yields a valid BOM with no components.
pub fn cbom_json() -> String {
    let components: Vec<serde_json::Value> = super::inventory().into_iter().map(component).collect();
    let doc = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "metadata": {
            "timestamp": format_rfc3339(jiff::Timestamp::now()),
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "xberg-native-pdf",
                    "version": env!("CARGO_PKG_VERSION"),
                }]
            }
        },
        "components": components,
    });
    // `serde_json::Value` always serialises; `to_string` is infallible
    // here (no non-string map keys, no NaN). ~keep
    serde_json::to_string(&doc).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::record_algorithm_use;

    #[test]
    fn rfc3339_timestamp_has_utc_offset_and_optional_fraction() {
        let whole = jiff::Timestamp::new(1_700_000_000, 0).unwrap();
        assert_eq!(format_rfc3339(whole), "2023-11-14T22:13:20+00:00");
        let frac = jiff::Timestamp::new(1_700_000_000, 123_456_789).unwrap();
        assert_eq!(format_rfc3339(frac), "2023-11-14T22:13:20.123456789+00:00");
    }

    #[test]
    fn cbom_timestamp_is_parseable_rfc3339() {
        let v: serde_json::Value = serde_json::from_str(&cbom_json()).expect("valid JSON");
        let ts = v["metadata"]["timestamp"].as_str().expect("timestamp string");
        assert!(ts.ends_with("+00:00"), "{ts}");
        ts.parse::<jiff::Timestamp>().expect("RFC 3339 timestamp");
    }

    #[test]
    fn cbom_is_well_formed_cyclonedx_listing_exercised_algorithms() {
        record_algorithm_use(AlgorithmId::HashSha256);
        record_algorithm_use(AlgorithmId::HashMd5);

        let json = cbom_json();
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["bomFormat"], "CycloneDX");
        assert_eq!(v["specVersion"], "1.6");
        assert_eq!(v["metadata"]["tools"]["components"][0]["name"], "xberg-native-pdf");

        let comps = v["components"].as_array().expect("components array");
        let by_name = |n: &str| {
            comps
                .iter()
                .find(|c| c["name"] == n)
                .unwrap_or_else(|| panic!("{n} present"))
                .clone()
        };

        let sha = by_name("sha256");
        assert_eq!(sha["type"], "cryptographic-asset");
        assert_eq!(sha["cryptoProperties"]["algorithmProperties"]["primitive"], "hash");
        assert_eq!(
            sha["cryptoProperties"]["algorithmProperties"]["certificationLevel"][0],
            "fips140-3"
        );

        let md5 = by_name("md5");
        assert_eq!(
            md5["cryptoProperties"]["algorithmProperties"]["certificationLevel"][0],
            "none"
        );

        for c in comps {
            assert_eq!(c["cryptoProperties"]["assetType"], "algorithm");
            assert!(c["bom-ref"].as_str().unwrap().starts_with("crypto/algorithm/"));
        }
    }
}
