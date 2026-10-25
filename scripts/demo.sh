#!/usr/bin/env bash
# Opens Tattile with sample drafts and reviews, in a temporary folder.
# Your real data in ~/Library/Application Support is not touched.
set -euo pipefail

cd "$(dirname "$0")/.."
DIR=$(mktemp -d -t tattile-demo)
cp demo/data.json "$DIR/data.json"
echo "Demo data folder: $DIR"
TATTILE_DATA_DIR="$DIR" cargo run --release
