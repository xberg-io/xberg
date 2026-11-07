use std::fs::{File, Metadata};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::control::WorkerControl;
use super::policy::invalid;
use super::{LocalClassificationConfig, LocalNliModel};

const MAX_GRAPH_BYTES: usize = 256 * 1024 * 1024;
const MAX_EXTERNAL_BYTES: usize = 256 * 1024 * 1024;
const MAX_TOKENIZER_BYTES: usize = 16 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_ARTIFACT_BYTES: usize = 512 * 1024 * 1024;
const COMPANIONS: [&str; 4] = [
    "tokenizer.json",
    "config.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
];

pub(super) struct VerifiedArtifacts {
    pub(super) graph: Vec<u8>,
    pub(super) external: Vec<(String, Vec<u8>)>,
    pub(super) tokenizer: Vec<u8>,
    pub(super) config: Vec<u8>,
    pub(super) tokenizer_config: Vec<u8>,
    pub(super) special_tokens: Vec<u8>,
}

fn validate_repository_path(path: &str) -> crate::Result<()> {
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path.bytes().any(|byte| byte.is_ascii_control())
        || path.split('/').any(|component| matches!(component, "" | "." | ".."))
        || Path::new(path).is_absolute()
    {
        return Err(invalid(
            "Local NLI external assets require normalized contained repository paths",
        ));
    }
    Ok(())
}

fn graph_relative_files(model: &LocalNliModel) -> crate::Result<Vec<(String, String)>> {
    validate_repository_path(&model.model_file)?;
    let directory = Path::new(&model.model_file).parent().unwrap_or_else(|| Path::new(""));
    model
        .external_files
        .iter()
        .map(|file| {
            validate_repository_path(file)?;
            let relative = Path::new(file)
                .strip_prefix(directory)
                .map_err(|_| invalid("Local NLI external assets must remain within the graph directory"))?;
            let name = relative
                .to_str()
                .ok_or_else(|| invalid("Local NLI external asset path must be UTF-8"))?;
            validate_repository_path(name)?;
            Ok((file.clone(), name.to_owned()))
        })
        .collect()
}

#[derive(Debug)]
struct Pin {
    path: String,
    sha256: String,
}

#[derive(Debug)]
pub(super) struct ArtifactInventory {
    graph: Pin,
    external: Vec<(String, Pin)>,
    companions: [Pin; COMPANIONS.len()],
}

impl ArtifactInventory {
    pub(super) fn model_sha256(&self) -> &str {
        &self.graph.sha256
    }

    pub(super) fn new(model: &LocalNliModel) -> crate::Result<Self> {
        let external_names = graph_relative_files(model)?;
        let mut pins: std::collections::BTreeMap<_, _> =
            crate::model_download::parse_sha256_manifest(&model.sha256_manifest)
                .map_err(invalid)?
                .into_iter()
                .collect();
        let graph = take_pin(&mut pins, &model.model_file)?;
        let external = external_names
            .into_iter()
            .map(|(remote, binding)| Ok((binding, take_pin(&mut pins, &remote)?)))
            .collect::<crate::Result<Vec<_>>>()?;
        let mut companions = Vec::with_capacity(COMPANIONS.len());
        for name in COMPANIONS {
            let path = companion_path(&pins, &model.model_file, name)?;
            companions.push(take_pin(&mut pins, &path)?);
        }
        if !pins.is_empty() {
            return Err(invalid("Local NLI manifest lists unexpected artifacts"));
        }
        let companions = companions
            .try_into()
            .map_err(|_| invalid("Local NLI companion inventory is incomplete"))?;
        Ok(Self {
            graph,
            external,
            companions,
        })
    }
}

type PinMap = std::collections::BTreeMap<String, String>;

fn take_pin(pins: &mut PinMap, path: &str) -> crate::Result<Pin> {
    let sha256 = pins
        .remove(path)
        .ok_or_else(|| invalid(format!("Local NLI artifact lacks a pin: {path}")))?;
    Ok(Pin {
        path: path.to_owned(),
        sha256,
    })
}

fn companion_path(pins: &PinMap, graph: &str, name: &str) -> crate::Result<String> {
    let nested = graph
        .rsplit_once('/')
        .map(|(directory, _)| format!("{directory}/{name}"))
        .filter(|path| pins.contains_key(path));
    match (pins.contains_key(name), nested) {
        (true, Some(_)) => Err(invalid(format!("Local NLI manifest has ambiguous pins for {name}"))),
        (true, None) => Ok(name.to_owned()),
        (false, Some(path)) => Ok(path),
        (false, None) => Err(invalid(format!("Local NLI artifact lacks a pin: {name}"))),
    }
}

pub(super) struct ArtifactBudget(usize);

impl ArtifactBudget {
    pub(super) fn new(bytes: usize) -> Self {
        Self(bytes)
    }

    #[cfg(test)]
    pub(super) fn remaining(&self) -> usize {
        self.0
    }
}

pub(super) fn read_verified(
    input_path: &Path,
    remote: &str,
    checksum: &str,
    cache_root: &Path,
    max_bytes: usize,
    budget: &mut ArtifactBudget,
) -> crate::Result<Vec<u8>> {
    let path = canonical_in_cache(input_path, cache_root)?;
    let mut file = open_file(&path, remote)?;
    let initial = file.metadata().map_err(|error| read_error(remote, error))?;
    if !initial.is_file() {
        return Err(invalid(format!("Local NLI artifact is not a regular file: {remote}")));
    }
    let length = usize::try_from(initial.len())
        .ok()
        .filter(|&length| length != 0 && length <= max_bytes && length <= budget.0)
        .ok_or_else(|| {
            invalid(format!(
                "Local NLI asset is empty or exceeds the remaining artifact byte limit: {remote}"
            ))
        })?;
    let bytes = read_exact_snapshot(&mut file, length, remote)?;
    let after = file.metadata().map_err(|error| read_error(remote, error))?;
    let current_path = canonical_in_cache(input_path, cache_root)?;
    let current = current_path.metadata().map_err(|error| read_error(remote, error))?;
    if current_path != path || !same_snapshot(&initial, &after) || !same_snapshot(&initial, &current) {
        return Err(changed_file(remote));
    }
    if hex::encode(Sha256::digest(&bytes)) != checksum {
        return Err(invalid(format!("Local NLI artifact checksum mismatch: {remote}")));
    }
    budget.0 -= length;
    Ok(bytes)
}

fn canonical_in_cache(path: &Path, cache_root: &Path) -> crate::Result<PathBuf> {
    let root = cache_root
        .canonicalize()
        .map_err(|error| invalid(format!("Local NLI pre-staged cache root is unavailable: {error}")))?;
    let path = path
        .canonicalize()
        .map_err(|error| invalid(format!("Local NLI cached asset path: {error}")))?;
    if !path.starts_with(root) {
        return Err(invalid("Local NLI asset escapes the configured cache root"));
    }
    Ok(path)
}

fn open_file(path: &Path, remote: &str) -> crate::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options.open(path).map_err(|error| read_error(remote, error))
}

pub(super) fn read_exact_snapshot(file: &mut File, len: usize, remote: &str) -> crate::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|error| invalid(format!("Local NLI artifact buffer allocation for {remote}: {error}")))?;
    bytes.resize(len, 0);
    file.read_exact(&mut bytes).map_err(|error| match error.kind() {
        ErrorKind::UnexpectedEof => changed_file(remote),
        _ => read_error(remote, error),
    })?;
    if file.read(&mut [0]).map_err(|error| read_error(remote, error))? != 0 {
        return Err(changed_file(remote));
    }
    Ok(bytes)
}

fn same_snapshot(first: &Metadata, second: &Metadata) -> bool {
    if !second.is_file() || first.len() != second.len() || first.modified().ok() != second.modified().ok() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        first.dev() == second.dev()
            && first.ino() == second.ino()
            && first.ctime() == second.ctime()
            && first.ctime_nsec() == second.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        first.created().ok() == second.created().ok()
    }
}

fn read_error(remote: &str, error: std::io::Error) -> crate::XbergError {
    invalid(format!("Local NLI cached artifact read for {remote}: {error}"))
}

fn changed_file(remote: &str) -> crate::XbergError {
    invalid(format!("Local NLI cached artifact changed while being read: {remote}"))
}

struct CachedReader<'a> {
    model: &'a LocalNliModel,
    control: &'a WorkerControl,
    client: hf_hub::HFClientSync,
    root: PathBuf,
    budget: ArtifactBudget,
}

impl CachedReader<'_> {
    fn read(&mut self, pin: &Pin, max_bytes: usize) -> crate::Result<Vec<u8>> {
        self.control.check()?;
        let path = crate::model_download::hf_cached_revision_with_client(
            &self.client,
            &self.model.repository,
            &pin.path,
            Some(&self.model.revision),
        )
        .map_err(invalid)?
        .ok_or_else(|| {
            invalid(format!(
                "Local NLI artifact is missing from the pre-staged cache: {}@{} {}",
                self.model.repository, self.model.revision, pin.path,
            ))
        })?;
        let bytes = read_verified(&path, &pin.path, &pin.sha256, &self.root, max_bytes, &mut self.budget)?;
        self.control.check()?;
        Ok(bytes)
    }
}

pub(super) fn read_artifacts(
    config: &LocalClassificationConfig,
    inventory: &ArtifactInventory,
    control: &WorkerControl,
) -> crate::Result<VerifiedArtifacts> {
    let cache_directory = config.cache_directory.as_deref().map(Path::new);
    let mut reader = CachedReader {
        model: &config.model,
        control,
        client: crate::model_download::hf_client(cache_directory).map_err(invalid)?,
        root: cache_directory
            .map(Path::to_path_buf)
            .unwrap_or_else(hf_hub::resolve_cache_dir),
        budget: ArtifactBudget::new(MAX_ARTIFACT_BYTES),
    };
    let [tokenizer, config, tokenizer_config, special_tokens] = &inventory.companions;
    Ok(VerifiedArtifacts {
        tokenizer: reader.read(tokenizer, MAX_TOKENIZER_BYTES)?,
        config: reader.read(config, MAX_METADATA_BYTES)?,
        tokenizer_config: reader.read(tokenizer_config, MAX_METADATA_BYTES)?,
        special_tokens: reader.read(special_tokens, MAX_METADATA_BYTES)?,
        graph: reader.read(&inventory.graph, MAX_GRAPH_BYTES)?,
        external: inventory
            .external
            .iter()
            .map(|(name, pin)| Ok((name.clone(), reader.read(pin, MAX_EXTERNAL_BYTES)?)))
            .collect::<crate::Result<Vec<_>>>()?,
    })
}
