use clap::Parser;

#[derive(Parser, Default, Debug)]
#[clap(version, author = "Milan Bolaric", about = "Convert Presentation Graphic Stream (SUP files) into SRT File", name = "pgs2srt")]
pub struct Args {
    #[clap(short, long)]
    pub pgs_file_name: String,

    #[clap(global = true, short, long, default_value = "eng")]
    pub language: String,

    #[clap(global = true, short, long, default_value = "")]
    pub srt_file_name: String
}