#!/usr/bin/env bash

set -euo pipefail

echo "=== Initial disk space ==="
df -h /

echo "=== Removing unnecessary packages ==="
sudo rm -rf /usr/share/dotnet
sudo rm -rf /usr/local/lib/android
sudo rm -rf /opt/ghc
sudo rm -rf /opt/hostedtoolcache/CodeQL
sudo rm -rf /usr/local/share/boost
sudo rm -rf /usr/local/lib/node_modules
sudo rm -rf /opt/microsoft

echo "=== Cleaning apt cache ==="
sudo apt-get clean
sudo rm -rf /var/lib/apt/lists/*

echo "=== Cleaning Docker ==="
docker system prune -af --volumes

echo "=== Available disk space after cleanup ==="
df -h /
