// Simple program to sort newly created files into appropriate subdirectories

use std::fs::{create_dir_all, rename};
use std::ops::Not;

use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify::event::CreateKind;
use notify_debouncer_full::{DebouncedEvent, new_debouncer};

const WATCH_DIR: &str = "/Users/brymes/Downloads";


fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();


    if let Err(error) = watch(WATCH_DIR) {
        log::error!("Error: {error:?}");
    }
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


    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| categorise(event)),
            Err(errors) => log::error!("{errors:?}")
        }
    }

    Ok(())
}

fn categorise(event: &DebouncedEvent) {
    if check_event(event.kind).not() {
        // Return Error or Ignore
        return;
    }

    let dir_name = get_dir_name(event.paths[0].to_str().unwrap());
    let file_name = event.paths[0].file_name().unwrap().to_string_lossy().to_string();
    let new_dir_full = format!("{}/{}/{}", WATCH_DIR.to_string(), dir_name.to_uppercase(), file_name.as_str());
    let new_dir = format!("{}/{}/", WATCH_DIR.to_string(), dir_name.to_uppercase());


    create_dir_all(new_dir.as_str()).expect("Unable to create Directories");
    /*
    TODO
    Function throws Error File Not Found because dispatching same event twice
    rename_res:Err(Os { code: 2, kind: NotFound, message: "No such file or directory" })
    MacOS Specific??
    */
    let rename_res = rename(event.paths[0].as_path(), &new_dir_full);

    //TODO Handle File already exists error
    println!("#########################################");
    println!("rename_res:{:?}", rename_res);
    if rename_res.is_err() {
        println!("rename_res_error:{:?}", rename_res.unwrap_err().kind());
    }
    println!("#########################################");

    return;
}

fn get_dir_name(path: &str) -> String {
    let extension = path.split(".").collect::<Vec<&str>>();
    if extension.len() < 2 {
        // Handle if file lacks extension
        return "scripts".to_string();
    } else if extension[0] == "." {
        // TODO find elegant way to handle hidden files
    }
    return extension.last().unwrap().to_string();
}

fn check_event(event_kind: EventKind) -> bool {
    match event_kind {
        EventKind::Create(CreateKind::File) => true,
        _ => false
    }
}

// fn create_week_dir() {}