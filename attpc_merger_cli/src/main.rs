//! # attpc_merger_cli
//!
//! Part of the attpc_merger crate family.
//!
//! This is the CLI application to merge AT-TPC data.
//!
//! ## Install
//!
//! Use `cargo install attpc_merger_cli`
//!
//! ## Use
//!
//! To merge data use the following command
//!
//! ```bash
//! attpc_merger_cli -p/--path <your_configuration.yaml>
//! ```
//!
//! To generate a configuration template file use
//!
//! ```bash
//! attpc_merger_cli -p/--path <your_configuration.yaml> new
//! ```
//!
//! ## Configuration
//!
//! The following fields must be specified in the configuration file:
//!
//! - graw_path: Specifies the full-path to a directory which contains the AT-TPC GETDAQ GRAW structure (i.e. contains subdirectories of the run_# format)
//! - evt_path: Specifies the full-path to a directory which contains the FRIBDAQ EVT structure (i.e. contains subdirectories of the run# format)
//! - hdf_path: Specifies the full-path to a directory to which merged HDF5 (.h5) files will be written
//! - pad_map_path: Specifies the full path to a CSV file which contains the mapping information for AT-TPC pads and electronics
//! - first_run_number: The starting run number (inclusive)
//! - last_run_number: The ending run number (inclusive)
//! - online: Boolean flag indicating if online data sources should be used (overrides some of the path imformation); generally should be false
//! - experiment: Experiment name as a string. Only used when online is true. Should match the experiment name used by the AT-TPC DAQ.
//! - n_threads: The number of worker threads to divide the merging amongst.

use clap::{Arg, Command};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libattpc_merger::config::Config;
//use libattpc_merger::process::{create_subsets, process, process_subset};
use libattpc_merger::process::{create_subsets, process_subset, ProcessStatus};

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

    println!("---------------------------- attpc_merger_cli ---------------------------");

    // Setup logging to a file
    let file_sink = Arc::new(
        spdlog::sink::FileSink::builder()
            .path(PathBuf::from("./attpc_merger_cli.log"))
            .formatter(Box::new(spdlog::formatter::PatternFormatter::new(
                spdlog::formatter::pattern!(
                    "[{date_short} {time_short}] - [thread: {tid}] - [{^{level}}] - {payload}{eol}"
                ),
            )))
            .truncate(true)
            .build()
            .unwrap(),
    );
    let logger = Arc::new(
        spdlog::Logger::builder()
            .flush_level_filter(spdlog::LevelFilter::All)
            .sink(file_sink)
            .build()
            .unwrap(),
    );
    spdlog::set_default_logger(logger);

    let pb_manager = MultiProgress::new();

    // Parse the cli
    let config_path = PathBuf::from(matches.get_one::<String>("path").expect("We require args"));

    if let Some(("new", _)) = matches.subcommand() {
        println!(
            "Making a template config at {}...",
            config_path.to_string_lossy()
        );

        make_template_config(&config_path);
        println!("Done.");
        println!("-------------------------------------------------------------------------");
        return;
    }

    // Load our config
    spdlog::info!("Loading config from {}...", config_path.display());
    let config = match Config::read_config_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            spdlog::error!("{e}");
            return;
        }
    };
    if !config.is_n_threads_valid() {
        spdlog::error!(
            "n_threads must be > 0 in config file {}",
            config_path.display()
        );
        println!(
            "n_threads must be > 0 in config file {}",
            config_path.display()
        );
        println!("-------------------------------------------------------------------------");
        return;
    }
    // Print out a bunch of info from the config as feedback to the user
    println!("Config successfully loaded.");
    println!("GRAW Path: {}", config.graw_path.to_string_lossy());
    println!("HDF5 Path: {}", config.hdf_path.to_string_lossy());
    println!("FRIB EVT Path: {}", config.evt_path.to_string_lossy());
    println!("PadMap Path: {:?}", config.pad_map_path);
    println!(
        "First Run: {} Last Run: {}",
        config.first_run_number, config.last_run_number
    );
    println!("Experiment Name: {}", config.experiment);
    println!("Is Online: {}", config.online);
    println!("Number of Worker Threads: {}", config.n_threads);
    println!("-------------------------- Progress Per Worker --------------------------");

    // Setup the progress bar, statuses, and workers
    let mut progress_bars = vec![];
    let mut statuses = vec![];
    let mut handles = vec![];

    // Split the runs into subsets for each worker
    let subsets = create_subsets(&config);
    spdlog::info!("Subsets: {subsets:?}");
    let mut error_occured = false;
    for (id, set) in subsets.into_iter().enumerate() {
        // Don't make a worker for no work!
        if set.is_empty() {
            continue;
        }
        // Create all of this worker's info
        let stat = Arc::new(Mutex::new(ProcessStatus {
            progress: 0.0,
            run_number: 0,
        }));
        let bar = pb_manager.add(
            ProgressBar::new(100)
                .with_style(
                    ProgressStyle::with_template(
                        "[{msg} - {ellapsed_precise}] {bar:40.cyan/blue} {percent}%",
                    )
                    .unwrap(),
                )
                .with_message(format!("Worker {id}: Run N/A")),
        );
        // Spawn it
        let conf = config.clone();
        progress_bars.push(bar);
        statuses.push(stat.clone());
        handles.push(std::thread::spawn(|| process_subset(conf, stat, set)))
    }

    loop {
        // Ugh since we don't have a UI here, I manually sleep for ~ 1 sec before trying to update
        std::thread::sleep(std::time::Duration::from_secs(1));
        // Update our progress bars with info from the workers
        for (idx, bar) in progress_bars.iter().enumerate() {
            let status = &statuses[idx];
            match status.lock() {
                Ok(stat) => {
                    bar.set_position((stat.progress * 100.0) as u64);
                    bar.set_message(format!("Worker {}: Run {}", idx, stat.run_number));
                }
                Err(e) => {
                    error_occured = true;
                    spdlog::error!("{e}");
                }
            }
        }

        // Critical: We exit the run loop if all of the workers are done
        let mut anyone_alive: bool = false;
        for handle in handles.iter_mut() {
            if !handle.is_finished() {
                anyone_alive = true;
                break;
            }
        }
        if !anyone_alive {
            break;
        }
    }

    // Recover all of our workers
    for handle in handles {
        match handle.join() {
            Ok(result) => match result {
                Ok(_) => spdlog::info!("Successfully merged data on one task!"),
                Err(e) => {
                    error_occured = true;
                    spdlog::error!("Merging failed with error: {e}")
                }
            },
            Err(_) => {
                error_occured = true;
                spdlog::error!("Failed to join merging task!")
            }
        }
    }

    // Shutdown the progress bars
    for bar in progress_bars {
        bar.finish();
    }
    println!("-------------------------------------------------------------------------");
    if error_occured {
        println!(
            "An error occurred during merging! Check the attpc_merger_cli.log file for details"
        )
    }

    println!("Done.");
    println!("-------------------------------------------------------------------------");
}
