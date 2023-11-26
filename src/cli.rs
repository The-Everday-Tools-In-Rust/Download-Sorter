use std::io::stdin;
use std::path::{Path, PathBuf};
use std::fs::metadata;
use std::ops::Deref;
use clap::Parser;

#[derive(Parser)]
#[command(about = "A CLI service to categorize files into folders based on their file extensions")]
#[command(author, version, long_about = None)]
struct Cli {
    /// Set path to watch
    #[arg(short, long, value_name = "FILE")]
    path: Option<PathBuf>,
}


fn parse_cli_args() -> &'static Path {
    let cli = Cli::parse();

    if let Some(config_path) = cli.path.as_deref().clone() {
        if config_path.is_absolute() && metadata(config_path).is_ok() {
            let target_path: &'static Path = config_path.clone();
            return target_path;
        }
    }
    parse_path_input()
}

pub fn parse_path_input() -> &'static Path {
    let mut path_input = String::new();

    println!("Welcome to FileSorter-Rs");
    loop {
        println!("Enter a valid macOS path to track: >>> ");

        match stdin().read_line(&mut path_input) {
            Ok(_) => {
                let parsed_path = path_input.trim().to_string();

                let target_path: &'static Path = Path::new(parsed_path.as_str());
                let is_valid_path = target_path.is_absolute() && metadata(parsed_path.as_str()).is_ok();

                if !is_valid_path {
                    println!("Error ::: Invalid Path supplied \n");
                } else {
                    return target_path;
                }
            }
            Err(error) => println!("error: {error}"),
        }
    }
}
