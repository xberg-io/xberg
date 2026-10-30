#!/usr/bin/env bash
#MISE description="Test the spring-ai integration against a locally built Java binding"
#USAGE flag "--binding-dir <dir>" env="BINDING_DIR" required=#true help="Directory holding the locally built Java binding and its Maven repository"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

mkdir -p target/release
cp "$usage_binding_dir"/java/libxberg_ffi.so target/release/
mvn -B -f integrations/java/spring-ai/pom.xml test \
  -Dmaven.repo.local="$usage_binding_dir/m2" \
  -DargLine="--enable-preview --enable-native-access=ALL-UNNAMED -XX:+EnableDynamicAgentLoading -Dnet.bytebuddy.experimental=true -Djava.library.path=${PWD}/target/release"
