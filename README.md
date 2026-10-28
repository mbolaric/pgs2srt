# pgs2srt

Convert Presentation Graphic Stream (SUP File) to text-based SRT format with Tesseract.

## Install

Download a package from the [releases page](https://github.com/mbolaric/pgs2srt/releases). Each one bundles Tesseract, Leptonica,
the English OCR model and the `pgs2srt` command-line tool, so nothing else has to be installed. Other OCR languages can be
downloaded from inside the desktop app.

| Platform                                                     | File                                                          | Notes                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------ | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 10/11 x64                                            | `pgs2srt-<version>-windows-x64-setup.exe` or `…-portable.zip` | The installer needs no administrator rights. The packages are not code-signed: on the SmartScreen warning choose _More info → Run anyway_.                                                                                                                       |
| macOS 15+ (Apple Silicon)                                    | `pgs2srt-<version>-macos-arm64.dmg`                           | Drag `pgs2srt.app` to Applications. The app is not notarized: on first launch open _System Settings → Privacy & Security_ and choose _Open Anyway_, or run `xattr -cr /Applications/pgs2srt.app`. The CLI is `/Applications/pgs2srt.app/Contents/MacOS/pgs2srt`. |
| Linux x86_64 (glibc 2.35+: Ubuntu 22.04, Debian 12 or newer) | `pgs2srt-<version>-linux-x86_64.AppImage`                     | `chmod +x` the file and run it. The CLI is in `pgs2srt-cli-<version>-linux-x86_64.tar.gz` (`bin/pgs2srt`).                                                                                                                                                       |

License notices of the bundled software are in the Windows install folder, in `pgs2srt.app/Contents/Resources` on macOS,
and under `usr/share/doc` in the AppImage (`share/doc` in the CLI archive).

## Web Version

The desktop app also runs in the browser at [mbolaric.github.io/pgs2srt](https://mbolaric.github.io/pgs2srt/), with OCR by
[tesseract.js](https://github.com/naptha/tesseract.js). Subtitle files never leave your computer: they are read and converted in
the browser. Finished SRTs wait in the queue until you download them, one at a time or all together with _Download All_ as one
`.zip` file. The first visit loads about 10 MB, and each OCR language about 4 MB more. The browser caches both. Conversion is slower
than in the desktop app, and folders, output folders and overwrite options are not available.

To host it yourself, unzip `pgs2srt-<version>-web.zip` from the [releases page](https://github.com/mbolaric/pgs2srt/releases)
into a folder of any static web server and open it over HTTP(S) (not as a local file). It works under any path. OCR languages
are still downloaded from GitHub on first use.

## Requirements (Ubuntu/Debian)

```bash
sudo apt install libleptonica-dev libtesseract-dev
```

### Install OCR Languages

```bash
sudo apt install tesseract-ocr-eng
sudo apt install tesseract-ocr-hrv
# or other languages: tesseract-ocr-<lang>
```

## Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/pgs2srt`.

## Usage

```bash
./target/release/pgs2srt -p <input.sup> [options]
```

### Options

- `-p, --pgs-file-name <FILE>`: Path to input PGS/SUP file (required).
- `-l, --language <LANG>`: Tesseract language code (default: `eng`), or `auto` to detect it from the file name tag and, failing that, from the subtitle images.
- `-s, --srt-file-name <FILE>`: Output `.srt` file path (optional, defaults to `<name>.<lang>.srt` next to the input file).
- `-f, --force`: Overwrite the output file if it already exists (by default an existing file is never replaced).

### Examples

Convert using default English OCR (creates `subtitles.eng.srt` next to `subtitles.sup`):

```bash
./target/release/pgs2srt -p subtitles.sup
```

Convert with specific language and output path:

```bash
./target/release/pgs2srt -p subtitles.sup -l hrv -s subtitles.srt
```

Detect the language automatically and replace an existing output:

```bash
./target/release/pgs2srt -p subtitles.sup -l auto -f
```

## Logs

Errors are written to a log file (rolled at 5 MB, 3 old files kept):

- Linux: `~/.local/state/pgs2srt/logs/` (or `$XDG_STATE_HOME/pgs2srt/logs/`)
- macOS: `~/Library/Logs/pgs2srt/`
- Windows: `%LOCALAPPDATA%\pgs2srt\logs\`

## Desktop App

The desktop app renders with wgpu (Metal / DX12 / Vulkan) and falls back to OpenGL (glow) if wgpu cannot start.
Set `PGS2SRT_RENDERER=glow` to force OpenGL.

On Linux Wayland desktops the app runs through XWayland, because winit (the windowing library) has no file drag and drop on native Wayland yet.
Set `PGS2SRT_BACKEND=wayland` to run as a native Wayland client instead (dragging files onto the window will not work; use Add files / Add folder).
