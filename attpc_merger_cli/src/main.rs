use clap::{Arg, Command};
use indicatif::{MultiProgress, ProgressBar};
use indicatif_log_bridge::LogWrapper;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libattpc_merger::config::Config;
use libattpc_merger::process::process;

fn make_template_config(path: &Path) {
    let config = Config::default();
    let yaml_str = serde_yaml::to_string(&config).unwrap();
    let mut file = File::create(path).expect("Could create template config file!");
    file.write_all(yaml_str.as_bytes())
        .expect("Failed to write yaml data to file!");
}

fn main() {
    // Create a cli
    let matches = Command::new("attpc_merger_cli")
        .arg_required_else_help(true)
        .subcommand(Command::new("new").about("Make a template configuration yaml file"))
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .help("Path to the file"),
        )
        .get_matches();

    // Initialize feedback
    let logger = simplelog::TermLogger::new(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    let pb_manager = MultiProgress::new();

    LogWrapper::new(pb_manager.clone(), logger)
        .try_init()
        .expect("Could not create logging/progress!");

    // Parse the cli
    let config_path = PathBuf::from(matches.get_one::<String>("path").expect("We require args"));

    match matches.subcommand() {
        Some(("new", _)) => {
            log::info!(
                "Making a template config at {}...",
                config_path.to_string_lossy()
            );

            make_template_config(&config_path);
            log::info!("Done.");
            return;
        }
        _ => (),
    }

    // Load our config
    log::info!("Loading config from {}...", config_path.to_string_lossy());
    let config = match Config::read_config_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("{e}");
            return;
        }
    };
    log::info!("Config successfully loaded.");
    log::info!("GRAW Path: {}", config.graw_path.to_string_lossy());
    log::info!("HDF5 Path: {}", config.hdf_path.to_string_lossy());
    log::info!("FRIB EVT Path: {}", config.evt_path.to_string_lossy());
    log::info!("PadMap Path: {}", config.pad_map_path.to_string_lossy());
    log::info!(
        "First Run: {} Last Run: {}",
        config.first_run_number,
        config.last_run_number
    );
    log::info!("Experiment Name: {}", config.experiment);
    log::info!("Is Online: {}", config.online);

    // Setup the progress bar
    let pb = pb_manager.add(ProgressBar::new(100));
    let status = Arc::new(Mutex::new(0.0));
    let sent_status = status.clone();
    // Spawn the task!
    let handle = std::thread::spawn(|| process(config, sent_status));

    loop {
        // Ugh since we don't have a UI here, I manually sleep for ~ 1 sec before trying to update
        std::thread::sleep(std::time::Duration::from_secs(1));
        match status.lock() {
            Ok(stat) => pb.set_position((*stat * 100.0) as u64),
            Err(e) => log::error!("{e}"),
        }

        if handle.is_finished() {
            match handle.join() {
                Ok(result) => match result {
                    Ok(_) => log::info!("Successfully merged data!"),
                    Err(e) => log::error!("Merging failed with error: {e}"),
                },
                Err(_) => log::error!("Failed to join merging task!"),
            }
            break;
        }
    }

    pb.finish();

    log::info!("Done.");
}
