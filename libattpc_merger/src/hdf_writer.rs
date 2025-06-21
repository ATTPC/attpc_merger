use hdf5::types::VarLenUnicode;
use hdf5::File;
use ndarray::Array2;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::error::HDF5WriterError;
use super::event::GetEvent;
use super::merger::Merger;
use super::ring_item::{PhysicsItem, RunInfo, ScalersItem};

const EVENTS_NAME: &str = "events";
const GET_NAME: &str = "get";
const GET_PADS_NAME: &str = "pads";
const GET_SI_UPFRONT_NAME: &str = "si_upstream_front";
const GET_SI_UPBACK_NAME: &str = "si_upstream_back";
const GET_SI_DOWNFRONT_NAME: &str = "si_downstream_front";
const GET_SI_DOWNBACK_NAME: &str = "si_downstream_back";
const SCALERS_NAME: &str = "scalers";
const FRIB_PHYSICS_NAME: &str = "frib_physics";

// All event counters start from 0 by law
const START_EVENT_NUMBER: u32 = 0;
/// This is the version of the output format
const FORMAT_VERSION: &str = "2.0";

/// A simple struct which wraps around the hdf5-rust library.
///
/// Opens an HDF5 file for writing merged Events. Currently writes
/// in the standard AT-TPC HDF5 format.
#[allow(dead_code)]
#[derive(Debug)]
pub struct HDFWriter {
    file_handle: File, //Idk if this needs to be kept alive, but I think it does
    parent_file_path: PathBuf,
    events_group: hdf5::Group,
    scalers_group: hdf5::Group,
    last_get_event: u64,    // GET final event number
    last_frib_event: u64,   // FRIB final event number
    last_scaler_event: u64, // FRIB scaler final event number
    first_timestamp: u64,   // GET info
    last_timestamp: u64,    // GET info
}
// Structure
// events - min_event, max_event, min_get_ts, max_get_ts, frib_run, frib_start, frib_stop, frib_time, version
// |---- event_#
// |    |---- get - id, timestamp, timestamp_other
// |    |    |---- pads(dset)
// |    |    |---- si_upstream_front(dset)
// |    |    |---- si_upstream_back(dset)
// |    |    |---- si_downstream_front(dset)
// |    |    |---- si_downstream_back(dset)
// |    |---- frib_physics - id, timestamp
// |    |    |---- 907(dset)
// |    |    |---- 1903(dset)
// scalers - min_event, max_event
// |---- event_#(dset) - start_offset, stop_offset, timestamp, incremental

impl HDFWriter {
    /// Create the writer, opening a file at path and creating the data groups
    pub fn new(path: &Path) -> Result<Self, HDF5WriterError> {
        let file_handle = File::create(path)?;
        let stem = path.parent().unwrap();
        let run_path = path.file_stem().unwrap();
        let parent_file_path = stem.join(format!("{}.yml", run_path.to_string_lossy()));

        let merger_version = format!("{}:{}", env!("CARGO_PKG_NAME"), FORMAT_VERSION);

        let events_group = file_handle.create_group(EVENTS_NAME)?;
        events_group.new_attr::<u64>().create("min_event")?;
        events_group.new_attr::<u64>().create("max_event")?;
        events_group.new_attr::<u64>().create("min_get_ts")?;
        events_group.new_attr::<u64>().create("max_get_ts")?;
        events_group.new_attr::<u32>().create("frib_run")?;
        events_group.new_attr::<u32>().create("frib_start")?;
        events_group.new_attr::<u32>().create("frib_stop")?;
        events_group.new_attr::<u32>().create("frib_time")?;
        events_group
            .new_attr::<hdf5::types::VarLenUnicode>()
            .create("version")?;
        events_group
            .attr("version")?
            .write_scalar(&VarLenUnicode::from_str(&merger_version).unwrap())?;

        let scalers_group = file_handle.create_group(SCALERS_NAME)?;
        scalers_group.new_attr::<u32>().create("min_event")?;
        scalers_group.new_attr::<u32>().create("max_event")?;
        scalers_group
            .new_attr::<hdf5::types::VarLenUnicode>()
            .create("version")?;
        scalers_group
            .attr("version")?
            .write_scalar(&VarLenUnicode::from_str(&merger_version).unwrap())?;

        Ok(Self {
            file_handle,
            parent_file_path,
            events_group,
            scalers_group,
            last_get_event: 0,
            last_frib_event: 0,
            last_scaler_event: 0,
            first_timestamp: 0,
            last_timestamp: 0,
        })
    }

    /// Write an event, where the event is converted into a data matrix
    pub fn write_get_event(
        &mut self,
        event: GetEvent,
        event_counter: &u64,
    ) -> Result<(), HDF5WriterError> {
        if *event_counter == (START_EVENT_NUMBER as u64) {
            // Catch first event ts
            self.first_timestamp = event.timestamp;
        }
        if *event_counter > self.last_get_event {
            self.last_get_event = *event_counter;
            self.last_timestamp = event.timestamp;
        }
        // copy to avoid borrow checker, ease of creating dataset
        let id = event.event_id;
        let ts = event.timestamp;
        let tso = event.timestampother;
        let event_name = format!("event_{}", event_counter);

        let event_group = match self.events_group.group(&event_name) {
            Ok(group) => group,
            Err(_) => self.events_group.create_group(&event_name)?,
        };
        let get_group = event_group.create_group(GET_NAME)?;
        let event_data = event.convert_to_data_matrices();
        get_group
            .new_dataset_builder()
            .with_data(&event_data.pad_matrix)
            .create(GET_PADS_NAME)?;
        get_group
            .new_dataset_builder()
            .with_data(&event_data.upstream_front_matrix)
            .create(GET_SI_UPFRONT_NAME)?;
        get_group
            .new_dataset_builder()
            .with_data(&event_data.upstream_back_matrix)
            .create(GET_SI_UPBACK_NAME)?;
        get_group
            .new_dataset_builder()
            .with_data(&event_data.downstream_front_matrix)
            .create(GET_SI_DOWNFRONT_NAME)?;
        get_group
            .new_dataset_builder()
            .with_data(&event_data.downstream_back_matrix)
            .create(GET_SI_DOWNBACK_NAME)?;
        get_group
            .new_attr::<u32>()
            .create("id")?
            .write_scalar(&id)?;
        get_group
            .new_attr::<u64>()
            .create("timestamp")?
            .write_scalar(&ts)?;
        get_group
            .new_attr::<u64>()
            .create("timestamp_other")?
            .write_scalar(&tso)?;

        Ok(())
    }

    /// Write graw file information in a separate yaml file
    pub fn write_fileinfo(&self, merger: &Merger) -> Result<(), HDF5WriterError> {
        let file_stacks = merger.get_file_stacks();
        let mut file_map = BTreeMap::<String, Vec<String>>::new();
        for stack in file_stacks.iter() {
            let file_name = format!(
                "cobo{}asad{}_file_names",
                stack.get_cobo_number(),
                stack.get_asad_number()
            );
            let size_name = format!(
                "cobo{}asad{}_file_sizes",
                stack.get_cobo_number(),
                stack.get_asad_number()
            );
            let file_stack = stack.get_file_stack_ref();
            let mut file_list = Vec::<String>::new();
            file_list.resize(file_stack.len() + 1, String::from(""));
            let mut size_list = file_list.clone();
            size_list[0] =
                human_bytes::human_bytes(stack.get_active_file().get_size_bytes() as f64); // Active file is the first one
            file_list[0] = String::from(stack.get_active_file().get_filename().to_string_lossy());
            for (row, path) in file_stack.iter().enumerate() {
                size_list[row + 1] =
                    human_bytes::human_bytes(path.metadata().unwrap().len() as f64);
                file_list[row + 1] = String::from(path.to_str().unwrap());
            }
            file_map.insert(file_name, file_list);
            file_map.insert(size_name, size_list);
        }

        let mut parent_file = std::fs::File::create(&self.parent_file_path)?;
        parent_file.write_all(serde_yaml::to_string(&file_map)?.as_bytes())?;

        Ok(())
    }

    /// Write meta information on first and last events, consume the writer
    pub fn close(self) -> Result<(), HDF5WriterError> {
        self.events_group
            .attr("min_event")?
            .write_scalar(&(START_EVENT_NUMBER as u64))?;
        self.events_group
            .attr("min_get_ts")?
            .write_scalar(&self.first_timestamp)?;
        // Check if FRIB & GET agree on event numbers
        if self.last_frib_event != self.last_get_event {
            spdlog::warn!("FRIB and GET do not agree on the number of events! FRIB saw {} events, while GET saw {} events", self.last_frib_event, self.last_get_event);
            spdlog::info!(
                "The max_event attribute of the event group will be set to the last GET event."
            );
        }
        self.events_group
            .attr("max_event")?
            .write_scalar(&self.last_get_event)?;
        self.events_group
            .attr("max_get_ts")?
            .write_scalar(&self.last_timestamp)?;
        self.scalers_group
            .attr("min_event")?
            .write_scalar(&START_EVENT_NUMBER)?;
        self.scalers_group
            .attr("max_event")?
            .write_scalar(&self.last_scaler_event)?;
        spdlog::info!(
            "{} events written. Run lasted {} seconds.",
            self.last_get_event,
            (self.last_timestamp - self.first_timestamp) / 100_000_000, // Time Stamp Clock is 100 MHz
        );
        Ok(())
    }

    /// Write meta information from evt file in frib group
    pub fn write_frib_runinfo(&self, run_info: RunInfo) -> Result<(), HDF5WriterError> {
        self.events_group
            .attr("frib_run")?
            .write_scalar(&run_info.begin.run)?;
        self.events_group
            .attr("frib_start")?
            .write_scalar(&run_info.begin.start)?;
        self.events_group
            .attr("frib_stop")?
            .write_scalar(&run_info.end.stop)?;
        self.events_group
            .attr("frib_time")?
            .write_scalar(&run_info.end.time)?;
        Ok(())
    }

    /// Write scaler data from evt file
    pub fn write_frib_scalers(
        &mut self,
        scalers: ScalersItem,
        counter: &u64,
    ) -> Result<(), HDF5WriterError> {
        if *counter > self.last_scaler_event {
            self.last_scaler_event = *counter;
        }
        let scaler_dset = self
            .scalers_group
            .new_dataset_builder()
            .with_data(&scalers.data)
            .create(format!("event_{}", counter).as_str())?;

        scaler_dset
            .new_attr::<u32>()
            .create("start_offset")?
            .write_scalar(&scalers.start_offset)?;
        scaler_dset
            .new_attr::<u32>()
            .create("stop_offset")?
            .write_scalar(&scalers.stop_offset)?;
        scaler_dset
            .new_attr::<u32>()
            .create("timestamp")?
            .write_scalar(&scalers.timestamp)?;
        scaler_dset
            .new_attr::<u32>()
            .create("incremental")?
            .write_scalar(&scalers.incremental)?;
        Ok(())
    }

    /// Write physics data from evt file
    pub fn write_frib_physics(
        &mut self,
        physics: PhysicsItem,
        event_counter: &u64,
    ) -> Result<(), HDF5WriterError> {
        // write attributes to event group
        if *event_counter > self.last_frib_event {
            self.last_frib_event = *event_counter;
        }

        let event_name = format!("event_{}", event_counter);
        let event_group = match self.events_group.group(&event_name) {
            Ok(group) => group,
            Err(_) => self.events_group.create_group(&event_name)?,
        };
        let physics_group = event_group.create_group(FRIB_PHYSICS_NAME)?;
        physics_group
            .new_attr::<u32>()
            .create("id")?
            .write_scalar(&physics.event)?;
        physics_group
            .new_attr::<u32>()
            .create("timestamp")?
            .write_scalar(&physics.timestamp)?;
        // write V977 data
        physics_group
            .new_dataset_builder()
            .with_data(&[physics.coinc.coinc])
            .create("977")?;
        // write SIS3300 data if present
        if physics.fadc1.hasdata == true {
            let mut data_matrix =
                Array2::<u16>::zeros([physics.fadc1.samples, physics.fadc1.traces.len()]);
            for i in 0..8 {
                for j in 0..physics.fadc1.samples {
                    data_matrix[[j, i]] = physics.fadc1.traces[i][j];
                }
            }
            physics_group
                .new_dataset_builder()
                .with_data(&data_matrix)
                .create("1903")?;
        }
        // write SIS3301 data if present
        if physics.fadc2.hasdata == true {
            let mut data_matrix =
                Array2::<u16>::zeros([physics.fadc2.samples, physics.fadc2.traces.len()]);
            for i in 0..8 {
                for j in 0..physics.fadc2.samples {
                    data_matrix[[j, i]] = physics.fadc2.traces[i][j];
                }
            }
            physics_group
                .new_dataset_builder()
                .with_data(&data_matrix)
                .create("1904")?;
        }
        // write SIS3301 data if present
        if physics.fadc3.hasdata == true {
            let mut data_matrix =
                Array2::<u16>::zeros([physics.fadc3.samples, physics.fadc3.traces.len()]);
            for i in 0..8 {
                for j in 0..physics.fadc3.samples {
                    data_matrix[[j, i]] = physics.fadc3.traces[i][j];
                }
            }
            physics_group
                .new_dataset_builder()
                .with_data(&data_matrix)
                .create("1905")?;
        }
        // write SIS3316 data if present (channel number is encoded as first element)
        if physics.fadc4.hasdata == true {
            let mut data_matrix =
                Array2::<u16>::zeros([physics.fadc4.samples+1, physics.fadc4.channels]);
            let mut index = 0;
            for i in 0..16 {
                if physics.fadc4.valid[i] == true {
                    for j in 0..physics.fadc4.samples+1 {
                        data_matrix[[j, index]] = physics.fadc4.traces[i][j];
                    }
                    index += 1;
                }
            }
            physics_group
                .new_dataset_builder()
                .with_data(&data_matrix)
                .create("1906")?;
        }
        Ok(())
    }
}
