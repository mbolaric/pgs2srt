#!/usr/bin/env bash
# Packages target/release into pgs2srt.app with the Homebrew Tesseract/Leptonica dylibs bundled, and wraps
# it in a DMG. The app needs the macOS version the Homebrew bottles were built for (the runner's) or newer.
# Usage: build-dmg.sh VERSION COMMON_DIR OUT_DIR
#   COMMON_DIR holds tessdata/, licenses/ and THIRD-PARTY-LICENSES.html from the release prepare job.
set -euo pipefail

version=$1
common=$(cd "$2" && pwd)
mkdir -p "$3"
out=$(cd "$3" && pwd)
root=$(cd "$(dirname "$0")/../.." && pwd)
work="$root/target/dmg"
dmg_dir="$work/dmg"
app="$dmg_dir/pgs2srt.app"
contents="$app/Contents"

rm -rf "$work"
mkdir -p "$contents/MacOS" "$contents/Resources"
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" >"$contents/Info.plist"
cp "$root/target/release/pgs2srt-desktop" "$root/target/release/pgs2srt" "$contents/MacOS/"
cp "$root/packaging/icons/pgs2srt.icns" "$root/LICENSE" "$common/THIRD-PARTY-LICENSES.html" "$contents/Resources/"
# Data must not live in Contents/MacOS (code signing); languages::data_path() also checks Resources.
cp -r "$common/tessdata" "$common/licenses" "$contents/Resources/"

dylibbundler --overwrite-dir --bundle-deps --create-dir \
    --fix-file "$contents/MacOS/pgs2srt-desktop" \
    --fix-file "$contents/MacOS/pgs2srt" \
    --dest-dir "$contents/Frameworks" \
    --install-path @executable_path/../Frameworks/ \
    --search-path "$(brew --prefix)/lib" </dev/null

# Copy the license files Homebrew keeps in the keg of every formula that provided a bundled dylib.
cellar=$(brew --cellar)
for dylib in "$contents"/Frameworks/*.dylib; do
    keg=$(find "$cellar" -maxdepth 4 -path "*/lib/$(basename "$dylib")" -print -quit | sed 's#/lib/[^/]*$##')
    if [[ -z "$keg" ]]; then
        echo "No Homebrew keg provides $(basename "$dylib")" >&2
        exit 1
    fi
    formula=$(basename "$(dirname "$keg")")
    target="$contents/Resources/licenses/$formula"
    [[ -d "$target" ]] && continue
    mkdir -p "$target"
    find "$keg" -maxdepth 1 -type f \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'COPYRIGHT*' -o -iname 'NOTICE*' \) \
        -exec cp {} "$target/" \;
    if [[ -z "$(ls -A "$target")" ]]; then
        brew info --json=v2 "$formula" |
            python3 -c 'import json, sys; f = json.load(sys.stdin)["formulae"][0]; print(f["name"], f["license"], f["homepage"], sep="\n")' \
                >"$target/LICENSE-INFO.txt"
    fi
done

# Ad-hoc signing seals the bundle resources, so Gatekeeper offers Open Anyway instead of reporting the app as
# damaged. Signed inside-out (Apple advises against --deep for signing); signing the bundle signs its main
# executable. The app is signed where hdiutil picks it up, so nothing touches it after signing.
codesign --force --sign - "$contents"/Frameworks/*.dylib "$contents/MacOS/pgs2srt"
codesign --force --sign - "$app"
codesign --verify --deep --strict --verbose=2 "$app"

ln -s /Applications "$dmg_dir/Applications"
# hdiutil sometimes fails with "Resource busy" on CI runners.
for attempt in 1 2 3; do
    hdiutil create -volname pgs2srt -srcfolder "$dmg_dir" -ov -format UDZO "$out/pgs2srt-$version-macos-arm64.dmg" && break
    [[ $attempt == 3 ]] && exit 1
    sleep 5
done
