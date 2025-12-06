use gtasa_extractor::parser::parse_path_file;
use std::fs::read_dir;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./paths")]
    path: String,
}

fn main() {
    let args = Args::parse();

    let mut node_files = Vec::new();
    let paths = read_dir(args.path).unwrap();
    for path in paths {
        let path = path.unwrap().path();
        node_files.push(parse_path_file(&path).unwrap());
    }
}
