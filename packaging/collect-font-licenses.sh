#!/usr/bin/env bash
# Copies the license of every font the UI bundles (crates/ui-egui/assets/fonts/<font>/OFL.txt) to
# OUT_DIR/<font>/OFL.txt.
# Usage: collect-font-licenses.sh OUT_DIR
set -euo pipefail

out=$1
fonts="$(cd "$(dirname "$0")/.." && pwd)/crates/ui-egui/assets/fonts"

for dir in "$fonts"/*/; do
    name=$(basename "$dir")
    mkdir -p "$out/$name"
    cp "$dir/OFL.txt" "$out/$name/"
done
