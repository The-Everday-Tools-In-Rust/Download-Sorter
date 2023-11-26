use std::fs::{create_dir_all, read_dir, rename};
use std::io::ErrorKind::{AlreadyExists, NotFound};
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::panic::catch_unwind;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::info;

use notify::{event::CreateKind, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{DebouncedEvent, new_debouncer};

pub fn watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_secs(5), None, tx).unwrap();

    // Add a path to be watched. All files and directories at that path and below will be monitored for changes.
    debouncer
        .watcher().watch(path.as_ref(), RecursiveMode::NonRecursive).unwrap();

    debouncer
        .cache()
        .add_root(path.as_ref(), RecursiveMode::NonRecursive);

    info!("Began watching selected Directory:  {path}", path = path.as_ref().display());
    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| categorise(event, path.as_ref())),
            Err(errors) => log::error!("{errors:?}")
        }
    };

    Ok(())
}


pub fn rename_and_move(watch_dir: &Path, file_path: &Path) {
    let new_dir = watch_dir.join(file_path.extension().unwrap().to_str().unwrap().to_uppercase());
    let destination_dir = new_dir.join(file_path.file_name().expect("Failure: Getting file name"));

    create_dir_all(new_dir).expect("Unable to create Directories");
    handle_rename(file_path, &destination_dir);
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

fn handle_rename(from_dir: &Path, to_dir: &PathBuf) {
    let rename_res = rename(from_dir, to_dir).map_err(|err| std::io::Error::new(err.kind(), err.to_string()));

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
