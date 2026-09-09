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
- `-l, --language <LANG>`: Tesseract language code (default: `eng`).
- `-s, --srt-file-name <FILE>`: Output `.srt` file path (optional, defaults to `<name>.<lang>.srt`).

### Examples
Convert using default English OCR (creates `subtitles.eng.srt`):
```bash
./target/release/pgs2srt -p subtitles.sup
```

Convert with specific language and output path:
```bash
./target/release/pgs2srt -p subtitles.sup -l hrv -s subtitles.srt
```
