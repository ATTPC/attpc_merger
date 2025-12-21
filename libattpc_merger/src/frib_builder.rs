use std::path::Path;

use super::error::FribBuilderError;
use super::evt_stack::EvtStack;
use super::hdf_writer::HDFWriter;
use super::ring_item::{BeginRunItem, EndRunItem, PhysicsItem, RingType, RunInfo, ScalersItem};

pub struct FribBuilder {
    evt_stack: EvtStack,
}

impl FribBuilder {
    pub fn new(evt_path: &Path) -> Result<Self, FribBuilderError> {
        Ok(Self {
            evt_stack: EvtStack::new(evt_path)?,
        })
    }

    pub fn process_evt_data(&mut self, writer: &mut HDFWriter) -> Result<(), FribBuilderError> {
        let mut run_info = RunInfo::new();
        let mut scaler_counter: u64 = 0;
        let mut event_counter: u64 = 0;
        while let Some(mut ring) = self.evt_stack.get_next_ring_item()? {
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
                                              //println!("fadc2 has {} samples", sam2);
                    writer.write_frib_physics(PhysicsItem::try_from(ring)?, &event_counter)?;
                    event_counter += 1;
                }
                RingType::Counter => (), // Unused, old that could cause many errors
                _ => spdlog::error!("Unrecognized ring type: {}", ring.bytes[4]),
            }
        }
        Ok(())
    }

    pub fn get_total_data_size(&self) -> u64 {
        self.evt_stack.total_stack_size_bytes
    }
}
