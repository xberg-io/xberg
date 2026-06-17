#!/usr/bin/env bash
set -euo pipefail

workdir="$(mktemp -d 2>/dev/null || mktemp -d -t fastembed)"
cd "$workdir"

cat >Cargo.toml <<'EOF'
[package]
name = "fastembed-prefetch"
version = "0.1.0"
edition = "2021"

[dependencies]
fastembed = { version = "FASTE_VER", default-features = false, features = ["hf-hub-native-tls", "ort-download-binaries"] }
EOF

if command -v gsed >/dev/null 2>&1; then
  gsed -i "s/FASTE_VER/${FASTEMBED_VERSION}/g" Cargo.toml
else
  sed -i.bak "s/FASTE_VER/${FASTEMBED_VERSION}/g" Cargo.toml || sed -i "s/FASTE_VER/${FASTEMBED_VERSION}/g" Cargo.toml
fi

mkdir -p src
cat >src/main.rs <<'EOF'
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

fn main() {
    let models = std::env::var("MODELS").unwrap_or_default();
    for raw in models.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if raw.eq_ignore_ascii_case("default") {
            let _ = TextEmbedding::try_new(Default::default());
            continue;
        }
        let model = match raw {
            "AllMiniLML6V2Q" => Some(EmbeddingModel::AllMiniLML6V2Q),
            "BGEBaseENV15" => Some(EmbeddingModel::BGEBaseENV15),
            "BGELargeENV15" => Some(EmbeddingModel::BGELargeENV15),
            "MultilingualE5Base" => Some(EmbeddingModel::MultilingualE5Base),
            _ => None,
        };
        if let Some(m) = model {
            let _ = TextEmbedding::try_new(InitOptions::new(m));
        } else {
            eprintln!("Skipping unknown model: {}", raw);
        }
    }
}
EOF

cargo run --release || true
