// Simple program to sort newly created files into appropriate subdirectories

mod cli;
mod file_watcher;
mod utils;

use std::env::current_exe;
use std::path::Path;
use std::thread;

use log::info;

const SERVICE_ID: &str = "homebrew.mxcl.file_sorter";

fn main() {
    let target_path = cli::parse_path_input();

    utils::configure_logging();

    let executable = current_exe().expect("Unable to Get Script Path");
    let binary_path = executable.to_str().expect("Unable to get Script Path");

    utils::create_and_load_mac_service(binary_path, SERVICE_ID, target_path.to_str().unwrap().as_ref());

    _ = thread::spawn(move || {
        let res = file_watcher::watch(Path::new(&target_path));
        match res {
            Ok(()) => {}
            Err(error) => info!("Error: {error:?}"),
        }
    });
}

