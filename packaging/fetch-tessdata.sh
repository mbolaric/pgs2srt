#!/usr/bin/env bash
# Downloads tessdata_fast models at the commit pinned in crates/pgs-core/models/models.tsv and verifies
# them like the in-app downloader. Writes OUT_DIR/tessdata/<lang>.traineddata and the repository license.
# Usage: fetch-tessdata.sh OUT_DIR LANG...
set -euo pipefail

out=$1
shift
models="$(cd "$(dirname "$0")/.." && pwd)/crates/pgs-core/models/models.tsv"
commit=$(awk 'NR == 1 { print $3 }' "$models")
base="https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$commit"

mkdir -p "$out/tessdata" "$out/licenses/tessdata_fast"
curl -fsSL --retry 3 -o "$out/licenses/tessdata_fast/LICENSE" "$base/LICENSE"

for lang in "$@"; do
    entry=$(awk -v lang="$lang" '$1 == lang { print $2, $3 }' "$models")
    if [[ -z "$entry" ]]; then
        echo "Unknown language: $lang" >&2
        exit 1
    fi
    read -r size sha <<<"$entry"
    file="$out/tessdata/$lang.traineddata"
    curl -fsSL --retry 3 -o "$file" "$base/$lang.traineddata"
    if [[ "$(wc -c <"$file" | tr -d ' ')" != "$size" || "$(git hash-object "$file")" != "$sha" ]]; then
        echo "$lang.traineddata does not match models.tsv" >&2
        exit 1
    fi
    echo "$lang.traineddata verified"
done
