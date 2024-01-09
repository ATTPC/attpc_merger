use hdf5::{types::VarLenAscii, File};
use ndarray::{Array1, Array2};
use std::error::Error;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use super::event::Event;
use super::merger::Merger;
use super::ring_item::{PhysicsItem, RunInfo, ScalersItem};

const GROUP_NAME: &str = "get";
const META_NAME: &str = "meta";
const FRIB_NAME: &str = "frib";
const EVT_NAME: &str = "evt";
const SCALER_NAME: &str = "scaler";

/// A simple struct which wraps around the hdf5-rust library.
///
/// Opens an HDF5 file for writing merged Events. Currently writes
/// in the standard AT-TPC HDF5 format.
#[allow(dead_code)]
#[derive(Debug)]
pub struct HDFWriter {
    file_handle: File, //Idk if this needs to be kept alive, but I think it does
    group: hdf5::Group,
    meta: hdf5::Group,
    meta_data: [u64; 4],
    frib: hdf5::Group,
    evt: hdf5::Group,
    scaler: hdf5::Group,
}

impl HDFWriter {
    /// Create the writer, opening a file at path and creating the data groups
    pub fn new(path: &Path) -> Result<Self, hdf5::Error> {
        let file_handle = File::create(path)?;
        let group = file_handle.create_group(GROUP_NAME)?;
        let meta = file_handle.create_group(META_NAME)?;
        let meta_data = [1000000000, 0, 0, 0];
        let frib = file_handle.create_group(FRIB_NAME)?;
        let evt = frib.create_group(EVT_NAME)?;
        let scaler = frib.create_group(SCALER_NAME)?;
        Ok(Self {
            file_handle,
            group,
            meta,
            meta_data,
            frib,
            evt,
            scaler,
        })
    }

    /// Write an event, where the event is converted into a data matrix
    pub fn write_event(&mut self, event: Event, event_counter: &u64) -> Result<(), hdf5::Error> {
        let header_builder = self.group.new_dataset_builder();
        let body_builder = self.group.new_dataset_builder();
        let event_body_name = format!("evt{}_data", event_counter);
        let event_header_name = format!("evt{}_header", event_counter);
        if *event_counter == 0 {
            // Catch first event
            self.meta_data[0] = *event_counter;
            self.meta_data[1] = event.timestamp;
        }
        if *event_counter > self.meta_data[2] {
            // Catch last event
            self.meta_data[2] = *event_counter;
            self.meta_data[3] = event.timestamp;
        }
        header_builder
            .with_data(&event.get_header_array())
            .create(event_header_name.as_str())?;
        body_builder
            .with_data(&event.convert_to_data_matrix())
            .create(event_body_name.as_str())?;
        Ok(())
    }

    /// Write graw file information in meta group
    pub fn write_fileinfo(&self, merger: &Merger) -> Result<(), Box<dyn Error>> {
        let file_stacks = merger.get_file_stacks();
        for stack in file_stacks.iter() {
            let file_builder = self.meta.new_dataset_builder();
            let size_builder = self.meta.new_dataset_builder();
            let file_name = format!(
                "cobo{}asad{}_files",
                stack.get_cobo_number(),
                stack.get_asad_number()
            );
            let size_name = format!(
                "cobo{}asad{}_length",
                stack.get_cobo_number(),
                stack.get_asad_number()
            );
            let file_stack = stack.get_file_stack_ref();
            let mut file_list = Array1::<VarLenAscii>::from_elem(
                file_stack.len() + 1,
                VarLenAscii::from_ascii("".as_bytes())?,
            );
            let mut size_list = Array1::<u64>::zeros([file_stack.len() + 1]);
            size_list[0] = stack.get_active_file().get_size_bytes(); // Active file is the first one
            file_list[0] = VarLenAscii::from_ascii(
                stack
                    .get_active_file()
                    .get_filename()
                    .to_str()
                    .unwrap()
                    .as_bytes(),
            )?;
            for (row, queue) in file_stack.iter().enumerate() {
                size_list[row + 1] = queue.metadata().unwrap().len();
                file_list[row + 1] =
                    VarLenAscii::from_ascii(queue.as_path().file_name().unwrap().as_bytes())?;
            }
            size_builder
                .with_data(&size_list)
                .create(size_name.as_str())?;
            file_builder
                .with_data(&file_list)
                .create(file_name.as_str())?;
        }
        Ok(())
    }

    /// Write meta information on first and last events
    pub fn write_meta(&self) -> Result<(), hdf5::Error> {
        let meta_builder = self.meta.new_dataset_builder();
        let meta_name = format!("meta");
        meta_builder
            .with_data(&self.meta_data)
            .create(meta_name.as_str())?;
        log::info!(
            "{} events written from {} to {}",
            self.meta_data[2] - self.meta_data[0],
            self.meta_data[0],
            self.meta_data[2]
        );
        log::info!(
            "Run lasted {} seconds",
            (self.meta_data[3] - self.meta_data[1]) / 100000000
        ); // Time Stamp Clock is 100 MHz
        Ok(())
    }

    /// Write meta information from evt file in frib group
    pub fn write_evtinfo(&self, run_info: RunInfo) -> Result<(), hdf5::Error> {
        let builder = self.frib.new_dataset_builder();
        let mut name = format!("runinfo");
        let data: [u32; 4] = [
            run_info.begin.run,
            run_info.begin.start,
            run_info.end.stop,
            run_info.end.time,
        ];
        builder.with_data(&data).create(name.as_str())?;
        name = format!("title");
        let builder = self.frib.new_dataset_builder();
        builder
            .with_data(&run_info.begin.title.as_str())
            .create(name.as_str())?;
        Ok(())
    }

    /// Write scaler data from evt file
    pub fn write_scalers(&self, scalers: ScalersItem, counter: u32) -> Result<(), hdf5::Error> {
        let builder = self.scaler.new_dataset_builder();
        let mut name = format!("scaler{}_header", counter);
        builder
            .with_data(&scalers.get_header_array())
            .create(name.as_str())?;
        name = format!("scaler{}_data", counter);
        let builder = self.scaler.new_dataset_builder();
        builder.with_data(&scalers.data).create(name.as_str())?;
        Ok(())
    }

    /// Write physics data from evt file
    pub fn write_physics(
        &self,
        physics: PhysicsItem,
        event_counter: &u64,
    ) -> Result<(), hdf5::Error> {
        // write header
        let builder = self.evt.new_dataset_builder();
        let mut name = format!("evt{}_header", event_counter);
        builder
            .with_data(&physics.get_header_array())
            .create(name.as_str())?;
        // write V977 data
        name = format!("evt{}_977", event_counter);
        let builder = self.evt.new_dataset_builder();
        let reg: [u16; 1] = [u16::try_from(physics.coinc.coinc).unwrap()];
        builder.with_data(&reg).create(name.as_str())?;
        // write SIS3300 data
        name = format!("evt{}_1903", event_counter);
        let builder = self.evt.new_dataset_builder();
        let mut data_matrix =
            Array2::<u16>::zeros([physics.fadc.samples, physics.fadc.traces.len()]);
        for i in 0..8 {
            for j in 0..physics.fadc.samples {
                data_matrix[[j, i]] = physics.fadc.traces[i][j];
            }
        }
        builder.with_data(&data_matrix).create(name.as_str())?;
        Ok(())
    }
}
