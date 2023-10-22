// Simple program to sort newly created files into appropriate subdirectories

use std::fs::{create_dir_all, rename};
use std::ops::Not;
use std::path::Path;
use std::sync::mpsc::channel;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::CreateKind;

const WATCH_DIR: &str = "/Users/brymes/Downloads";


fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    /*    let path = std::env::args()
            .nth(1)
            .expect("Argument 1 needs to be a path");

        log::info!("Watching {path}");*/

    if let Err(error) = watch(WATCH_DIR) {
        log::error!("Error: {error:?}");
    }
}

fn watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = channel();

    // No specific tickrate, max debounce time 1 seconds
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;


    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    // let mut watcher = RecommendedWatcher::new(tx, Config::default())?;


    for res in rx {
        match res {
            Ok(event) => {
                log::info!("Change: {event:?}");
                categorise(event);
            }
            Err(error) => log::error!("Error: {error:?}"),
        }
    }

    Ok(())
}

fn categorise(event: Event) {
    if check_event(event.kind).not() {
        // Return Error or Ignore
        return;
    }


    let dir_name = get_dir_name(event.paths[0].to_str().unwrap());
    let file_name = event.paths[0].file_name().unwrap().to_string_lossy().to_string();
    let new_dir_full = format!("{}/{}/{}", WATCH_DIR.to_string(), dir_name.to_uppercase(), file_name.as_str());
    let new_dir = format!("{}/{}/", WATCH_DIR.to_string(), dir_name.to_uppercase());

    println!("#########################################");
    println!("#           DEBUG INFO                  #");
    println!("#########################################");
    println!("debug:{:?}", event.paths[0].as_path());
    println!("debug:{:?}", dir_name);
    println!("debug:{:?}", new_dir);
    println!("#########################################");
    println!("debug:{:?}", new_dir_full);

    create_dir_all(new_dir.as_str()).unwrap();
    let rename_res = rename(event.paths[0].as_path(), &new_dir_full);

    println!("#########################################");
    println!("#########################################");
    println!("debug:{:?}", rename_res);
    println!("debug:{:?}", rename_res.is_err());
    // println!("debug:{:?}", rename_res.unwrap_err());
    // println!("debug:{:?}", rename_res.unwrap_err().kind());

    return;
}

fn get_dir_name(path: &str) -> String {
    // let file = path.split("/").collect::<Vec<&str>>();
    let extension = path.split(".").collect::<Vec<&str>>();
    return extension.last().unwrap().to_string();
}

fn check_event(event_kind: EventKind) -> bool {
    match event_kind {
        EventKind::Create(CreateKind::File) => true,
        _ => false
    }
}