use std::path::PathBuf;

use super::constants::{NUMBER_OF_ASADS, NUMBER_OF_COBOS, COBO_OF_SILICON};
use super::error::AsadStackError;

use super::asad_stack::AsadStack;
use super::config::Config;
use super::error::MergerError;
use super::graw_frame::GrawFrame;

/// The object which merges all of the data from individual .graw files into a single data stream.
///
/// Merger essentially performs a merge-sort operation on the data files, taking all of the separate
/// data from the .graw files and zipping them into a single data stream which is sorted in time.
/// Currently uses EventID to decide the time of a frame, not the timestamp.
#[derive(Debug)]
pub struct Merger {
    file_stacks: Vec<AsadStack>,
    total_data_size_bytes: u64,
}

impl Merger {
    /// Create a new merger. Requires the path to the graw data files
    pub fn new(config: &Config, run_number: i32) -> Result<Self, MergerError> {
        let mut merger = Merger {
            file_stacks: Vec::new(),
            total_data_size_bytes: 0,
        };

        //For every asad in every cobo, attempt to make a stack
        let mut graw_dir: PathBuf;
        for cobo in 0..NUMBER_OF_COBOS {
            if !config.merge_atttpc && cobo < COBO_OF_SILICON {
                continue;
            }
            if !config.merge_silicon && cobo >= COBO_OF_SILICON {
                continue;
            }
            if config.need_copy_files() {
                graw_dir = config.get_copy_directory(run_number)?.unwrap()
                    .join("graw")
                    .join(format!("mm{cobo}"));
            } else {
                if config.online {
                    graw_dir = config.get_online_directory(run_number, &cobo)?;
                } else {
                    graw_dir = config.get_run_directory(run_number, &cobo)?;
                }
            }
            for asad in 0..NUMBER_OF_ASADS {
                match AsadStack::new(&graw_dir, cobo as i32, asad as i32) {
                    Ok(stack) => {
                        merger.file_stacks.push(stack);
                    }
                    Err(AsadStackError::NoMatchingFiles) => {
                        continue;
                    }
                    Err(e) => {
                        return Err(MergerError::AsadError(e));
                    }
                }
            }
        }

        //Oops no files
        if merger.file_stacks.is_empty() {
            return Err(MergerError::NoFilesError);
        }

        merger.total_data_size_bytes = merger
            .file_stacks
            .iter()
            .fold(0, |sum, stack| sum + stack.get_stack_size_bytes());
        Ok(merger)
    }

    /// Asks the stacks for the next frame.
    ///
    /// Which ever stack has the earliest event, returns its frame.
    /// Returns `Result<Option<GrawFrame>>`. If the Option is None, that
    /// means that there is no more data to be read from the stacks
    pub fn get_next_frame(&mut self) -> Result<Option<GrawFrame>, MergerError> {
        let mut earliest_event_index: Option<(usize, u32)> = Option::None;
        for (idx, stack) in self.file_stacks.iter_mut().enumerate() {
            if let Some(meta) = stack.get_next_frame_metadata()? {
                match earliest_event_index {
                    None => {
                        earliest_event_index = Some((idx, meta.event_id));
                    }
                    Some((_index, event_id)) => {
                        if meta.event_id < event_id {
                            earliest_event_index = Some((idx, meta.event_id));
                        }
                    }
                }
            }
        }

        if earliest_event_index.is_none() {
            //None of the remaining stacks had data for us. We've read everything.
            Ok(None)
        } else {
            //This MUST happen before the retain call. The indexes will be modified.
            let frame = self.file_stacks[earliest_event_index.unwrap().0].get_next_frame()?;
            //Only keep stacks which still have data to be read
            self.file_stacks.retain(|stack| stack.is_not_ended());
            Ok(Some(frame))
        }
    }

    /// Total size of the run in bytes
    pub fn get_total_data_size(&self) -> &u64 {
        &self.total_data_size_bytes
    }

    /// Get an immutable reference to the underlying file stacks
    pub fn get_file_stacks(&self) -> &Vec<AsadStack> {
        &self.file_stacks
    }
}
