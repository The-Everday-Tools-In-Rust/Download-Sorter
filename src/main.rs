// Simple program to sort newly created files into appropriate subdirectories

mod cli;

use std::env::current_exe;
use std::fs::{create_dir_all, File, read_dir, rename};
use std::io::{ErrorKind::{AlreadyExists, NotFound}, Write};
use std::ops::Not;
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread;

use log::{info, LevelFilter};

use chrono::Utc;

use error_chain::{ChainedError, error_chain};

use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

use notify::{event::CreateKind, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{DebouncedEvent, new_debouncer};

const PLIST_PATH: &str = "~/Library/LaunchAgents";
const SERVICE_ID: &str = "homebrew.mxcl.downloads_sorter";
const BASE_LOG_PATH: &str = "~/Library/Logs/Homebrew/file_sorter_logs";

fn main() {
    configure_logging();
    let executable = current_exe().expect("Unable to Get Script Path");
    let binary_path = executable.to_str().expect("Unable to get Script Path");

    create_and_load_mac_service(binary_path, SERVICE_ID);

    let pth = cli::parse_path_input();

    _ = thread::spawn(move || {
        let res = watch(Path::new(&pth));
        match res {
            Ok(()) => {}
            Err(error) => info!("Error: {error:?}"),
        }
    });
}

fn configure_logging() {
    create_dir_all(Path::new(shellexpand::tilde(BASE_LOG_PATH).as_ref())).expect("Unable to create Log Directories");

    let full_log_path = format!("{}/sorter_{}.txt", BASE_LOG_PATH, Utc::now().format("%Y%m%d_%H%M%S"));
    let log_path_extended = shellexpand::tilde(&full_log_path);
    let log_path = Path::new(log_path_extended.as_ref());

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(log_path).expect("Error(001) Initializing logger");

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder()
            .appender("logfile")
            .build(LevelFilter::Info)).expect("Error(002) Initializing logger");

    log4rs::init_config(config).expect("Error(003) Initializing logger");
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

    info!("Began watching selected Directory:  {path}", path = path.as_ref().display());
    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| categorise(event, path.as_ref())),
            Err(errors) => log::error!("{errors:?}")
        }
    };

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


fn create_and_load_mac_service(binary_path: &str, service_id: &str) {
    // Define the path for the plist file
    let plist_path = format!("{}/{}.plist", PLIST_PATH, service_id);
    let expanded_plist_path = shellexpand::tilde(&plist_path);

    // Create the plist content
    let plist_content = format!(r#"<?xml version="1.0" encoding="UTF]]]]]]]-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{service_id}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#, service_id = service_id, binary_path = binary_path);

    // Use launchctl to load the plist
    if Path::new(&*expanded_plist_path).exists() {
        Command::new("launchctl")
            .args(&["load", "-w", &expanded_plist_path.to_string()])
            .output().expect("Error running launchtl");
    } else {
        // Write the plist file
        let mut file = File::create(&*expanded_plist_path).unwrap();
        let _ = file.write_all(plist_content.as_bytes());
        _ = create_and_load_mac_service(binary_path, service_id);
    }
}



error_chain! {
    foreign_links {
        Io(std::io::Error);
        LogConfig(log4rs::config::Errors);
        SetLogger(log::SetLoggerError);
    }
}