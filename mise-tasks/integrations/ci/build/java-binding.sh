#!/usr/bin/env bash
#MISE description="Build the local Java binding for integration tests"
set -euo pipefail

cargo build --release -p xberg-ffi
(
  cd packages/java
  mvn -B install \
    -DskipTests \
    -Dpmd.skip=true \
    -Dcpd.skip=true \
    -Dmaven.javadoc.skip=true \
    -Dsource.skip=true \
    -Dmaven.repo.local="${PWD}/../../dist/m2"
)
mkdir -p dist/java
cp target/release/libxberg_ffi.so dist/java/
