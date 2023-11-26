use std::fs::{create_dir_all, File, metadata};
use std::path::Path;

use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log::LevelFilter;
use std::process::Command;
use std::io::Write;
use chrono::Utc;

use error_chain::{error_chain};


const BASE_LOG_PATH: &str = "~/Library/Logs/Homebrew/file_sorter_logs";
const PLIST_PATH: &str = "~/Library/LaunchAgents";

error_chain! {
    foreign_links {
        Io(std::io::Error);
        LogConfig(log4rs::config::Errors);
        SetLogger(log::SetLoggerError);
    }
}

pub fn configure_logging() {
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


pub fn create_and_load_mac_service(binary_path: &str, service_id: &str, target_path: &str) {
    // Define the path for the plist file
    let plist_path = format!("{}/{}.plist", PLIST_PATH, service_id);
    let expanded_plist_path = shellexpand::tilde(&plist_path);

    if metadata(expanded_plist_path.trim()).is_ok() {
        return;
    }

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
        <string>path</string>
        <string>{target_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#, service_id = service_id, binary_path = binary_path, target_path = target_path);

    // Use launchctl to load the plist
    if Path::new(&*expanded_plist_path).exists() {
        Command::new("launchctl")
            .args(&["load", "-w", &expanded_plist_path.to_string()])
            .output().expect("Error running launchtl");
    } else {
        // Write the plist file
        let mut file = File::create(&*expanded_plist_path).unwrap();
        let _ = file.write_all(plist_content.as_bytes());
        _ = create_and_load_mac_service(binary_path, service_id, target_path);
    }
}
