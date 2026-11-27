//! Identity of the Tesseract language packs an Xberg run resolves, so two result sets can be
//! compared for the models they were measured on, not only for the model set they requested.

use std::collections::{BTreeSet, HashMap};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ProvenanceInputs;
use crate::Result;
use crate::ocr_preflight::{
    DEFAULT_OCR_LANGUAGE, TessdataEnvironment, candidate_tessdata_dirs_for, first_dir_holding, requested_languages,
};

/// One `<language>.traineddata` file as found on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TessdataPackProvenance {
    pub language: String,
    pub bytes: u64,
    pub sha256: String,
}

/// Where one OCR language request resolves, and the packs it loads from there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TessdataRequestProvenance {
    /// The request as Xberg receives it: language codes joined by `+`.
    pub languages: String,
    /// The first directory in Xberg's search order that holds every pack of the request, as a
    /// path-free label: `TESSDATA_PREFIX`, `<cache>/tessdata`, `<cache>/tessdata/<model>`, or a
    /// system or image directory. `None` when no searched directory holds them all, in which case
    /// Xberg downloads the pinned packs of the requested set on first use.
    pub directory: Option<String>,
    /// One entry per language of the request, in request order; empty when unresolved.
    pub packs: Vec<TessdataPackProvenance>,
}

/// The Tesseract packs behind a run's Xberg Tesseract rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TessdataProvenance {
    /// The set the run asked for (`--tessdata-model`). The packs below are what was on disk; they
    /// are not classified as `fast` or `best`.
    pub requested_model: xberg::TessdataModel,
    /// One entry per distinct OCR language request among the fixtures those rows run, sorted.
    pub requests: Vec<TessdataRequestProvenance>,
}

/// Whether a framework is an Xberg row that can run Tesseract, and therefore reads tessdata.
fn reads_tessdata(framework: &str) -> bool {
    framework.starts_with("xberg-") && !framework.contains("paddle") && !framework.contains("sceptre")
}

/// The OCR language request a fixture makes of Xberg; an unpinned fixture gets Xberg's default.
fn language_request(ocr_language: Option<&str>) -> String {
    let codes: Vec<&str> = ocr_language.map(requested_languages).into_iter().flatten().collect();
    if codes.is_empty() {
        DEFAULT_OCR_LANGUAGE.to_string()
    } else {
        codes.join("+")
    }
}

fn eligible_requests(inputs: &ProvenanceInputs<'_>) -> BTreeSet<String> {
    let mut requests = BTreeSet::new();
    for adapter in inputs
        .frameworks
        .iter()
        .filter(|adapter| reads_tessdata(adapter.name()))
    {
        for (_, fixture) in inputs.fixtures.fixtures() {
            if adapter.supports_fixture(
                &fixture.file_type,
                fixture.document.file_name().and_then(|name| name.to_str()),
                fixture.ocr_language(),
            ) {
                requests.insert(language_request(fixture.ocr_language()));
            }
        }
    }
    requests
}

/// Capture the packs the run's Xberg Tesseract rows resolve from the harness environment, which
/// the Xberg processes inherit. `None` when no such row has a fixture to run.
pub(super) fn capture(inputs: &ProvenanceInputs<'_>) -> Result<Option<TessdataProvenance>> {
    let requests = eligible_requests(inputs);
    if requests.is_empty() {
        return Ok(None);
    }
    let requested_model = inputs.config.tessdata_model;
    let environment = TessdataEnvironment::from_process();
    let dirs = candidate_tessdata_dirs_for(requested_model, &environment);
    let requests = resolve_requests(
        &requests,
        &dirs,
        &|dir: &Path| environment.directory_label(dir),
        &mut pack_size_and_sha256,
    )?;
    Ok(Some(TessdataProvenance {
        requested_model,
        requests,
    }))
}

/// Resolve each request against `dirs` the way Xberg does and identify its packs, reading every
/// file once however many requests share it.
fn resolve_requests(
    requests: &BTreeSet<String>,
    dirs: &[PathBuf],
    label: &dyn Fn(&Path) -> String,
    identify: &mut dyn FnMut(&Path) -> Result<(u64, String)>,
) -> Result<Vec<TessdataRequestProvenance>> {
    let mut identified: HashMap<PathBuf, (u64, String)> = HashMap::new();
    let mut resolved = Vec::with_capacity(requests.len());
    for request in requests {
        let languages: Vec<&str> = requested_languages(request).collect();
        let Some(dir) = first_dir_holding(&languages, dirs) else {
            resolved.push(TessdataRequestProvenance {
                languages: request.clone(),
                directory: None,
                packs: Vec::new(),
            });
            continue;
        };
        let mut packs = Vec::with_capacity(languages.len());
        for language in languages {
            let path = dir.join(format!("{language}.traineddata"));
            let (bytes, sha256) = match identified.get(&path) {
                Some(identity) => identity.clone(),
                None => {
                    let identity = identify(&path)?;
                    identified.insert(path, identity.clone());
                    identity
                }
            };
            packs.push(TessdataPackProvenance {
                language: language.to_string(),
                bytes,
                sha256,
            });
        }
        resolved.push(TessdataRequestProvenance {
            languages: request.clone(),
            directory: Some(label(dir)),
            packs,
        });
    }
    Ok(resolved)
}

/// The byte length and lowercase hex SHA-256 of a file, the identity Xberg pins its packs by.
fn pack_size_and_sha256(path: &Path) -> Result<(u64, String)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    let sha256 = hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect();
    Ok((bytes, sha256))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const ABCD_SHA256: &str = "88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589";

    fn dir_with(packs: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (language, content) in packs {
            std::fs::write(dir.path().join(format!("{language}.traineddata")), content).unwrap();
        }
        dir
    }

    fn requests(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn pack(language: &str, bytes: u64, sha256: &str) -> TessdataPackProvenance {
        TessdataPackProvenance {
            language: language.to_string(),
            bytes,
            sha256: sha256.to_string(),
        }
    }

    #[test]
    fn a_pack_is_identified_by_its_size_and_sha256() {
        let dir = dir_with(&[("eng", "abc")]);
        assert_eq!(
            pack_size_and_sha256(&dir.path().join("eng.traineddata")).unwrap(),
            (3, ABC_SHA256.to_string())
        );
        assert!(pack_size_and_sha256(&dir.path().join("deu.traineddata")).is_err());
    }

    #[test]
    fn each_request_records_the_first_directory_holding_all_of_its_packs() {
        let first = dir_with(&[("eng", "abc")]);
        let second = dir_with(&[("eng", "abcd"), ("deu", "abc")]);
        let dirs = [first.path().to_path_buf(), second.path().to_path_buf()];
        let label = |dir: &Path| {
            if dir == first.path() {
                "first".to_string()
            } else {
                "second".to_string()
            }
        };

        let resolved = resolve_requests(
            &requests(&["eng", "deu+eng", "fra"]),
            &dirs,
            &label,
            &mut pack_size_and_sha256,
        )
        .unwrap();

        assert_eq!(
            resolved,
            [
                TessdataRequestProvenance {
                    languages: "deu+eng".to_string(),
                    directory: Some("second".to_string()),
                    packs: vec![pack("deu", 3, ABC_SHA256), pack("eng", 4, ABCD_SHA256)],
                },
                TessdataRequestProvenance {
                    languages: "eng".to_string(),
                    directory: Some("first".to_string()),
                    packs: vec![pack("eng", 3, ABC_SHA256)],
                },
                TessdataRequestProvenance {
                    languages: "fra".to_string(),
                    directory: None,
                    packs: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn a_pack_shared_by_several_requests_is_read_once() {
        let dir = dir_with(&[("eng", "abc"), ("deu", "abc"), ("fra", "abc")]);
        let dirs = [dir.path().to_path_buf()];
        let mut reads: Vec<PathBuf> = Vec::new();
        let mut identify = |path: &Path| {
            reads.push(path.to_path_buf());
            pack_size_and_sha256(path)
        };

        let resolved = resolve_requests(
            &requests(&["eng", "deu+eng", "eng+fra"]),
            &dirs,
            &|_: &Path| "dir".to_string(),
            &mut identify,
        )
        .unwrap();

        assert_eq!(resolved.iter().map(|request| request.packs.len()).sum::<usize>(), 5);
        reads.sort();
        assert_eq!(
            reads,
            ["deu", "eng", "fra"].map(|language| dir.path().join(format!("{language}.traineddata")))
        );
    }

    #[test]
    fn an_unpinned_or_blank_fixture_requests_the_default_language() {
        assert_eq!(language_request(None), "eng");
        assert_eq!(language_request(Some(" + ")), "eng");
        assert_eq!(language_request(Some(" deu + eng ")), "deu+eng");
    }

    #[test]
    fn only_xberg_rows_that_can_run_tesseract_read_tessdata() {
        for framework in [
            "xberg-markdown-baseline",
            "xberg-markdown-layout-batch",
            "xberg-markdown-auto-ocr",
            "xberg-rust-steady-state",
        ] {
            assert!(reads_tessdata(framework), "{framework}");
        }
        for framework in [
            "xberg-markdown-paddle-ocr",
            "xberg-markdown-auto-ocr-paddle",
            "xberg-markdown-sceptre-ort",
            "docling",
            "tesseract",
        ] {
            assert!(!reads_tessdata(framework), "{framework}");
        }
    }
}
