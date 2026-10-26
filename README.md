# pgs2srt
Convert Presentation Graphic Stream (SUP File) to text-based SRT format with Tesseract.  
  
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

## Desktop renderer
The desktop app renders with wgpu (Metal / DX12 / Vulkan) and falls back to OpenGL (glow) if wgpu cannot start.
Set `PGS2SRT_RENDERER=glow` to force OpenGL.

On Linux Wayland desktops the app runs through XWayland, because winit (the windowing library) has no file drag and drop on native Wayland yet.
Set `PGS2SRT_BACKEND=wayland` to run as a native Wayland client instead (dragging files onto the window will not work; use Add files / Add folder).
