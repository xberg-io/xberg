#!/usr/bin/env bash

# A simple build script
set -euo pipefail

BUILD_DIR="./build"

build() {
    echo "Building project..."
    mkdir -p "$BUILD_DIR"
    echo "Done."
}

clean() {
    echo "Cleaning..."
    rm -rf "$BUILD_DIR"
}

build
