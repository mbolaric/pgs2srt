use clap::Parser;

#[derive(Parser, Default, Debug)]
#[clap(
    version,
    author = "Milan Bolaric",
    about = "Convert Presentation Graphic Stream (SUP files) into SRT File",
    name = "pgs2srt"
)]
pub struct Args {
    #[clap(short, long)]
    pub pgs_file_name: String,

    /// Tesseract language code, or `auto` to detect it from the file name and subtitle images.
    #[clap(global = true, short, long, default_value = "eng")]
    pub language: String,

    /// Output path; defaults to `<input dir>/<input name>.<lang>.srt`.
    #[clap(global = true, short, long, default_value = "")]
    pub srt_file_name: String,

    /// Overwrite the output file if it already exists.
    #[clap(global = true, short, long)]
    pub force: bool,
}
