#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
TMP=$(mktemp -d)
cd "$TMP"
curl -fsSL https://github.com/grafana/k6/releases/download/v0.47.0/k6-v0.47.0-linux-amd64.tar.gz -o k6.tgz
tar -xzf k6.tgz
mv k6-*/k6 ../../k6
echo "k6 downloaded to ./k6"
