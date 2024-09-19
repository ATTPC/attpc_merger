use fxhash::FxHashMap;
use ndarray::{s, Array1, Array2};

use super::constants::*;
use super::error::EventError;
use super::graw_frame::GrawFrame;
use super::pad_map::{HardwareID, PadMap};

/// # Event
/// An event is a collection of traces which all occured with the same Event ID generated by the AT-TPC DAQ.
/// An event is created from a Vec of GrawFrames, which are then parsed into ndarray traces. The event can also subtract
/// the fixed pattern noise recored by the electronics. To write the event to HDF5, convert the event to a data matrix.
#[derive(Debug)]
pub struct Event {
    nframes: i32,
    traces: FxHashMap<HardwareID, Array1<i16>>, //maps pad id to the trace for that pad
    pub timestamp: u64,
    pub timestampother: u64,
    pub event_id: u32,
}

impl Event {
    /// Make a new event from a list of GrawFrames
    pub fn new(pad_map: &PadMap, frames: &Vec<GrawFrame>) -> Result<Self, EventError> {
        let mut event = Event {
            nframes: 0,
            traces: FxHashMap::default(),
            timestamp: 0,
            timestampother: 0,
            event_id: 0,
        };
        for frame in frames {
            event.append_frame(pad_map, frame)?;
        }

        Ok(event)
    }

    /// Convert the event traces to a data matrix for writing to disk. Follows format used by AT-TPC analysis
    pub fn convert_to_data_matrix(self) -> Array2<i16> {
        let mut data_matrix = Array2::<i16>::zeros([self.traces.len(), NUMBER_OF_MATRIX_COLUMNS]);
        for (row, (hw_id, trace)) in self.traces.into_iter().enumerate() {
            data_matrix[[row, 0]] = hw_id.cobo_id as i16;
            data_matrix[[row, 1]] = hw_id.asad_id as i16;
            data_matrix[[row, 2]] = hw_id.aget_id as i16;
            data_matrix[[row, 3]] = hw_id.channel as i16;
            data_matrix[[row, 4]] = hw_id.pad_id as i16;
            let mut trace_slice = data_matrix.slice_mut(s![row, 5..NUMBER_OF_MATRIX_COLUMNS]);
            trace.move_into(&mut trace_slice);
        }

        data_matrix
    }

    // Formated header array
    // Now unused
    // pub fn get_header_array(&self) -> Array1<f64> {
    //     ndarray::arr1(&[
    //         self.event_id as f64,
    //         self.timestamp as f64,
    //         self.timestampother as f64,
    //     ])
    // }

    /// Add a frame to the event.
    ///
    /// If the frame does not belong to this event, an error is returned
    fn append_frame(&mut self, pad_map: &PadMap, frame: &GrawFrame) -> Result<(), EventError> {
        // Check if this is the first frame or that the event id's match
        if self.nframes == 0 {
            self.event_id = frame.header.event_id;
        } else if self.event_id != frame.header.event_id {
            return Err(EventError::MismatchedEventID(
                frame.header.event_id,
                self.event_id,
            ));
        }

        if frame.header.cobo_id == COBO_WITH_TIMESTAMP {
            // this cobo has a TS in sync with other DAQ
            self.timestampother = frame.header.event_time;
        } else {
            // all other cobos have the same TS from Mutant
            self.timestamp = frame.header.event_time;
        }

        let mut hw_id: &HardwareID;
        for datum in frame.data.iter() {
            // Reject FPN channels
            if FPN_CHANNELS.contains(&datum.channel) {
                continue;
            }

            // Get the hardware ID
            hw_id = match pad_map.get_hardware_id(
                &frame.header.cobo_id,
                &frame.header.asad_id,
                &datum.aget_id,
                &datum.channel,
            ) {
                Some(hw) => hw,
                None => {
                    continue;
                }
            };

            // Put the data in the appropriate trace
            match self.traces.get_mut(hw_id) {
                Some(trace) => {
                    trace[datum.time_bucket_id as usize] = datum.sample;
                }
                None => {
                    //First time this pad found during event. Create a new array
                    let mut trace: Array1<i16> =
                        Array1::<i16>::zeros(NUMBER_OF_TIME_BUCKETS as usize);
                    trace[datum.time_bucket_id as usize] = datum.sample;
                    self.traces.insert(hw_id.clone(), trace);
                }
            }
        }

        self.nframes += 1;

        Ok(())
    }
}
