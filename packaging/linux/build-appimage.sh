#!/usr/bin/env bash
# Packages target/release into an AppImage and a CLI tarball with Tesseract, Leptonica and their shared
# libraries bundled. Build on the oldest distro you want to support: the result needs its glibc or newer.
# Usage: build-appimage.sh VERSION COMMON_DIR OUT_DIR
#   COMMON_DIR holds tessdata/, licenses/ and THIRD-PARTY-LICENSES.html from the release prepare job.
set -euo pipefail

version=$1
common=$(realpath "$2")
mkdir -p "$3"
out=$(realpath "$3")
root=$(cd "$(dirname "$0")/../.." && pwd)
work="$root/target/appimage"
tools="$root/target/tools"
appdir="$work/AppDir"
export APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64

fetch_tool() {
    if [[ ! -x "$tools/$1" ]]; then
        mkdir -p "$tools"
        curl -fsSL --retry 3 -o "$tools/$1" "$2"
        chmod +x "$tools/$1"
    fi
}
fetch_tool linuxdeploy https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage
fetch_tool appimagetool https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage

rm -rf "$work"
mkdir -p "$work"
"$tools/linuxdeploy" --appdir "$appdir" \
    --executable "$root/target/release/pgs2srt-desktop" \
    --executable "$root/target/release/pgs2srt" \
    --desktop-file "$root/packaging/linux/pgs2srt.desktop" \
    --icon-file "$root/packaging/icons/pgs2srt.png"

# languages::data_path() looks for tessdata next to the executable.
cp -r "$common/tessdata" "$appdir/usr/bin/"

doc="$appdir/usr/share/doc"
mkdir -p "$doc/pgs2srt"
cp "$root/LICENSE" "$common/THIRD-PARTY-LICENSES.html" "$doc/pgs2srt/"
cp -r "$common/licenses/." "$doc/"

# Every bundled library needs its copyright notice, and the LGPL ones a pointer to their source.
packages=()
for lib in "$appdir"/usr/lib/*.so*; do
    name=$(basename "$lib")
    package=$({ dpkg -S "*/$name" 2>/dev/null || true; } | head -n 1 | cut -d: -f1)
    copyright="/usr/share/doc/$package/copyright"
    if [[ -z "$package" || ! -f "$copyright" ]]; then
        echo "No copyright file found for $name" >&2
        exit 1
    fi
    mkdir -p "$doc/$package"
    cp "$copyright" "$doc/$package/copyright"
    packages+=("$package")
done
{
    echo "Source packages of the bundled shared libraries (Ubuntu $(lsb_release -rs) and ppa:alex-p/tesseract-ocr5)."
    echo "Get them with 'apt-get source <package>=<version>', from https://launchpad.net/ubuntu/+source/<package>"
    echo "or from https://launchpad.net/~alex-p/+archive/ubuntu/tesseract-ocr5."
    echo
    dpkg-query -W -f='${source:Package} ${source:Version}\n' "${packages[@]}" | sort -u
} >"$doc/pgs2srt/SOURCES.txt"

"$tools/appimagetool" --no-appstream "$appdir" "$out/pgs2srt-$version-linux-x86_64.AppImage"

# The CLI shares the AppDir libraries; linuxdeploy already set its RUNPATH to $ORIGIN/../lib.
cli="pgs2srt-cli-$version-linux-x86_64"
mkdir -p "$work/$cli/bin" "$work/$cli/share"
cp -a "$appdir/usr/bin/pgs2srt" "$appdir/usr/bin/tessdata" "$work/$cli/bin/"
cp -a "$appdir/usr/lib" "$work/$cli/"
cp -a "$doc" "$work/$cli/share/"
tar -C "$work" -czf "$out/$cli.tar.gz" "$cli"
