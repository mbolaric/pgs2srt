#!/usr/bin/env bash
# Downloads the pinned tesseract.js and tesseract.js-core npm packages, verifies their npm integrity hashes
# and writes what the web app serves: OUT_DIR/tesseract/ (scripts and LSTM cores) and OUT_DIR/licenses/
# (tesseract.js, tesseract.js-core and the C libraries compiled into the core, at their submodule commits).
# Usage: fetch-tesseract-js.sh OUT_DIR
set -euo pipefail

out=$1
version=7.0.0
packages=(
    "tesseract.js sha512-exPBkd+z+wM1BuMkx/Bjv43OeLBxhL5kKWsz/9JY+DXcXdiBjiAch0V49QR3oAJqCaL5qURE0vx9Eo+G5YE7mA=="
    "tesseract.js-core sha512-WnNH518NzmbSq9zgTPeoF8c+xmilS8rFIl1YKbk/ptuuc7p6cLNELNuPAzcmsYw450ca6bLa8j3t0VAtq435Vw=="
)
core_licenses=(
    "tesseract https://raw.githubusercontent.com/Balearica/tesseract/2a9c1c49c360462733c386d2a44fcd22c4e21411/LICENSE"
    "leptonica https://raw.githubusercontent.com/DanBloomberg/leptonica/4af068b56a9674da915debea4ed7e1b9885b17e8/leptonica-license.txt"
    "libjpeg https://raw.githubusercontent.com/LuaDist/libjpeg/6c0fcb8ddee365e7abc4d332662b06900612e923/README"
    "giflib https://raw.githubusercontent.com/mirrorer/giflib/fa37672085ce4b3d62c51627ab3c8cf2dda8009a/COPYING"
    "libpng https://raw.githubusercontent.com/glennrp/libpng/a37d4836519517bdce6cb9d956092321eca3e73b/LICENSE"
    "libtiff https://gitlab.com/libtiff/libtiff/-/raw/b51bb157123264e26d34c09cc673d213aea61fc7/COPYRIGHT"
    "libwebp https://raw.githubusercontent.com/webmproject/libwebp/20ef03ee351d4ff03fc5ff3ec4804a879d1b9d5c/COPYING"
    "libwebp https://raw.githubusercontent.com/webmproject/libwebp/20ef03ee351d4ff03fc5ff3ec4804a879d1b9d5c/PATENTS"
    "zlib https://raw.githubusercontent.com/madler/zlib/21767c654d31d2dccdde4330529775c6c5fd5389/README"
    "openlibm https://raw.githubusercontent.com/JuliaMath/openlibm/ae2d91698508701c83cab83714d42a1146dccf85/LICENSE.md"
)

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for entry in "${packages[@]}"; do
    read -r name integrity <<<"$entry"
    archive="$work/$name.tgz"
    curl -fsSL --retry 3 -o "$archive" "https://registry.npmjs.org/$name/-/$name-$version.tgz"
    if [[ "sha512-$(openssl dgst -sha512 -binary "$archive" | base64 | tr -d '\n')" != "$integrity" ]]; then
        echo "$name-$version.tgz does not match its npm integrity hash" >&2
        exit 1
    fi
    mkdir -p "$work/$name"
    tar -xzf "$archive" -C "$work/$name" --strip-components=1
    echo "$name $version verified"
done

mkdir -p "$out/tesseract/core" "$out/licenses/tesseract.js" "$out/licenses/tesseract.js-core"
cp "$work/tesseract.js/dist/tesseract.min.js" "$work/tesseract.js/dist/worker.min.js" "$out/tesseract/"
# The worker loads one of these by the browser's SIMD support; OEM 1 (LSTM only) never needs the others.
cp "$work/tesseract.js-core/"tesseract-core{,-simd,-relaxedsimd}-lstm.wasm.js "$out/tesseract/core/"
cp "$work/tesseract.js/LICENSE.md" "$work/tesseract.js/dist/"*.LICENSE.txt "$out/licenses/tesseract.js/"
cp "$work/tesseract.js-core/LICENSE" "$out/licenses/tesseract.js-core/"

for entry in "${core_licenses[@]}"; do
    read -r name url <<<"$entry"
    mkdir -p "$out/licenses/$name"
    curl -fsSL --retry 3 -o "$out/licenses/$name/${url##*/}" "$url"
done
