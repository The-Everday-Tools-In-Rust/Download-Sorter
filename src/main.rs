// Simple program to sort newly created files into appropriate subdirectories

use std::{env, fs, io};
use std::fs::{create_dir_all, rename, read_dir};
use std::io::Error;
use std::ops::Not;

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::io::ErrorKind::{NotFound, AlreadyExists};
use std::panic::catch_unwind;
use log::info;

use notify::{EventKind, RecursiveMode, Watcher};
use notify::event::CreateKind;
use notify_debouncer_full::{DebouncedEvent, new_debouncer};


fn main() {
    let watch_dir = parse_cli_args();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let watch_dir = Path::new(&watch_dir);
    // categorize_existing_files(watch_dir);

    if let Err(error) = watch(watch_dir) {
        log::error!("Error: {error:?}");
    }
}

fn parse_cli_args() -> String {
    let mut path_input = String::new();

    loop {
        println!("Enter a valid macOS path: >>> ");

        match io::stdin().read_line(&mut path_input) {
            Ok(_) => {
                path_input = path_input.trim().to_string();

                if Path::new(&path_input).is_absolute() && fs::metadata(&path_input).is_ok() {
                    break;
                } else {
                    println!("Invalid path: '{}'", path_input);
                }
            }
            Err(error) => println!("error: {error}"),
        }
    }

    return path_input;
}

fn watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_secs(5), None, tx).unwrap();

    // Add a path to be watched. All files and directories at that path and below will be monitored for changes.
    debouncer
        .watcher().watch(path.as_ref(), RecursiveMode::NonRecursive).unwrap();

    // Initialize the file id cache for the same path. This will allow the debouncer to stitch together move events,
    // even if the underlying watch implementation doesn't support it.
    // Without the cache and with some watch implementations,
    // you may receive `move from` and `move to` events instead of one `move both` event.
    debouncer
        .cache()
        .add_root(path.as_ref(), RecursiveMode::NonRecursive);

    info!("Began watching selected Directory:  {}", path.as_ref().display());
    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| categorise(event, path.as_ref())),
            Err(errors) => log::error!("{errors:?}")
        }
    }

    Ok(())
}

fn categorize_existing_files(watch_dir: &Path) {
    for entry in read_dir(watch_dir).expect("Error reading Existing files") {
        let entry = entry.expect("Error reading existing files");
        let path = entry.path();

        if path.is_file() {
            rename_and_move(watch_dir, &path);
        }
    }
}

fn categorise(event: &DebouncedEvent, watch_dir: &Path) {
    if check_event(event.kind).not() {
        // Return Error or Ignore
        return;
    }
    catch_unwind(|| rename_and_move(watch_dir, &event.paths[0].as_path())).unwrap_or_else(|_| {
        // Handle panic here, log it, and continue execution
        info!("Error renaming file: {}", event.paths[0].as_path().display());
    });

    return;
}


fn check_event(event_kind: EventKind) -> bool {
    match event_kind {
        EventKind::Create(CreateKind::File) => true,
        _ => false
    }
}


fn rename_and_move(watch_dir: &Path, file_path: &Path) {
    let new_dir = watch_dir.join(file_path.extension().unwrap().to_str().unwrap().to_uppercase());
    let destination_dir = new_dir.join(file_path.file_name().expect("Failure: Getting file name"));

    create_dir_all(new_dir).expect("Unable to create Directories");
    handle_rename(file_path, &destination_dir);
}

fn handle_rename(from_dir: &Path, to_dir: &PathBuf) {
    let rename_res = rename(from_dir, to_dir).map_err(|err| Error::new(err.kind(), err.to_string()));

    match rename_res {
        Ok(()) => {}
        Err(ref error) => {
            info!("#########################################");
            info!("rename_res:{:?}", rename_res);
            match error.kind() {
                /*
                TODO
                Function throws Error File Not Found because dispatching same event twice
                rename_res:Err(Os { code: 2, kind: NotFound, message: "No such file or directory" })
                MacOS Specific??
                */
                NotFound => info!("File not found: {}", from_dir.display()),
                AlreadyExists => info!("File already exists: {}", to_dir.display()),
                _ => info!("Error renaming file: {}", error),
            }
            info!("#########################################");
        }
    }
}

