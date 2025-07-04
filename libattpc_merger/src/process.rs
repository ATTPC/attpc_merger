use std::path::PathBuf;
use std::sync::mpsc::Sender;

use super::ring_item::{BeginRunItem, EndRunItem, PhysicsItem, RingType, RunInfo, ScalersItem};

use super::channel_map::GetChannelMap;
use super::config::Config;
use super::constants::SIZE_UNIT;
use super::error::ProcessorError;
use super::event_builder::EventBuilder;
use super::evt_stack::EvtStack;
use super::hdf_writer::HDFWriter;
use super::merger::Merger;
use super::worker_status::WorkerStatus;

/// The final event of the EventBuilder will need a manual flush
fn flush_final_event(
    mut evb: EventBuilder,
    mut writer: HDFWriter,
    event_counter: &u64,
) -> Result<(), ProcessorError> {
    if let Some(event) = evb.flush_final_event() {
        writer.write_get_event(event, event_counter)?;
    } else {
        spdlog::warn!("Last event was not flushed successfully!")
    }
    writer.close()?;
    Ok(())
}

/// Process the evt data for this run
fn process_evt_data(evt_path: PathBuf, writer: &mut HDFWriter) -> Result<(), ProcessorError> {
    let mut evt_stack = EvtStack::new(&evt_path)?; // open evt file
    let mut run_info = RunInfo::new();
    let mut scaler_counter: u64 = 0;
    let mut event_counter: u64 = 0;
    while let Some(mut ring) = evt_stack.get_next_ring_item()? {
        match ring.ring_type {
            // process each ring depending on its type
            RingType::BeginRun => {
                // Begin run
                run_info.begin = BeginRunItem::try_from(ring)?;
                spdlog::info!("Detected begin run -- {}", run_info.print_begin());
            }
            RingType::EndRun => {
                // End run
                run_info.end = EndRunItem::try_from(ring)?;
                spdlog::info!("Detected end run -- {}", run_info.print_end());
                writer.write_frib_runinfo(run_info)?;
                break;
            }
            RingType::Dummy => (),
            RingType::Scalers => {
                // Scalers
                writer.write_frib_scalers(ScalersItem::try_from(ring)?, &scaler_counter)?;
                scaler_counter += 1;
            }
            RingType::Physics => {
                // Physics data
                ring.remove_boundaries(); // physics event often cross VMUSB buffer boundary
                writer.write_frib_physics(PhysicsItem::try_from(ring)?, &event_counter)?;
                event_counter += 1;
            }
            RingType::Counter => (), // Unused, old that could cause many errors
            _ => spdlog::error!("Unrecognized ring type: {}", ring.bytes[4]),
        }
    }
    Ok(())
}

/// The main loop of attpc_merger.
///
/// This takes in a config (and progress monitor) and preforms the merging logic on the recieved data.
pub fn process_run(
    config: &Config,
    run_number: i32,
    tx: &Sender<WorkerStatus>,
    worker_id: &usize,
) -> Result<(), ProcessorError> {
    let hdf_path = config.get_hdf_file_name(run_number)?;
    let pad_map = GetChannelMap::new(config.channel_map_path.as_deref())?;

    //Initialize the merger, event builder, and hdf writer
    let mut merger = Merger::new(config, run_number)?;
    spdlog::info!(
        "Total run size: {}",
        human_bytes::human_bytes(*merger.get_total_data_size() as f64)
    );
    let mut evb = EventBuilder::new(pad_map);
    let mut writer = HDFWriter::new(&hdf_path)?;

    let total_data_size = merger.get_total_data_size();
    let flush_frac: f32 = 0.01;
    let mut count = 0;
    let mut progress: f32 = 0.0;
    let flush_val = (*total_data_size as f64 * flush_frac as f64) as u64;

    // Handle evt data if present
    match config.get_evt_directory(run_number) {
        Ok(evt_path) => {
            spdlog::info!("Now processing evt data...");
            match process_evt_data(evt_path, &mut writer) {
                Ok(_) => spdlog::info!("Done with evt data."),
                Err(e) => {
                    spdlog::warn!("Error while processing evt data: {e}\nSkipping evt processing.")
                }
            }
        }
        Err(e) => {
            spdlog::warn!("Could not access evt directory: {e}");
            spdlog::warn!("Skipping processing evt data...");
        }
    }

    //Handle the get data
    spdlog::info!("Processing get data...");
    writer.write_fileinfo(&merger)?;
    let mut event_counter = 0;
    loop {
        if let Some(frame) = merger.get_next_frame()? {
            //Merger found a frame
            //bleh
            count += (frame.header.frame_size * SIZE_UNIT) as u64;
            if count > flush_val {
                count = 0;
                progress += flush_frac;
                tx.send(WorkerStatus::new(progress, run_number, *worker_id))?;
            }

            if let Some(event) = evb.append_frame(frame)? {
                writer.write_get_event(event, &event_counter)?;
                event_counter += 1;
            } else {
                continue;
            }
        } else {
            //If the merger returns none, there is no more data to be read
            flush_final_event(evb, writer, &event_counter)?;
            break;
        }
    }

    tx.send(WorkerStatus::new(1.0, run_number, *worker_id))?;
    spdlog::info!("Done with get data.");

    Ok(())
}

/// The function to be called by a separate thread (typically the UI).
/// This particular flavor is unused by the default tools (attpc_merger and attpc_merger_cli)
/// but could be useful to someone else
/// Allows multiple runs to be processed
pub fn process(
    config: Config,
    tx: Sender<WorkerStatus>,
    worker_id: usize,
) -> Result<(), ProcessorError> {
    for run in config.first_run_number..(config.last_run_number + 1) {
        tx.send(WorkerStatus::new(0.0, run, worker_id))?;
        if config.does_run_exist(run) {
            spdlog::info!("Processing run {}...", run);
            process_run(&config, run, &tx, &worker_id)?;
            spdlog::info!("Finished processing run {}.", run);
        } else {
            spdlog::info!("Run {} does not exist, skipping...", run);
        }
    }
    Ok(())
}

/// Process a subset of runs
pub fn process_subset(
    config: Config,
    tx: Sender<WorkerStatus>,
    worker_id: usize,
    subset: Vec<i32>,
) -> Result<(), ProcessorError> {
    for run in subset {
        tx.send(WorkerStatus::new(0.0, run, worker_id))?;
        if config.does_run_exist(run) {
            spdlog::info!("Processing run {}...", run);
            process_run(&config, run, &tx, &worker_id)?;
            spdlog::info!("Finished processing run {}.", run);
        } else {
            spdlog::info!("Run {} does not exist, skipping...", run);
        }
    }
    Ok(())
}

/// Divide a run range in to a set of subranges (per thread/worker)
pub fn create_subsets(config: &Config) -> Vec<Vec<i32>> {
    let mut subsets: Vec<Vec<i32>> = vec![Vec::new(); config.n_threads as usize];
    let n_subsets = subsets.len();

    for (idx, run) in (config.first_run_number..(config.last_run_number + 1)).enumerate() {
        subsets[idx % n_subsets].push(run)
    }

    subsets
}
