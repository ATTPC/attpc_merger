use super::error::{EvtFileError, EvtStackError};
use super::evt_file::EvtFile;
use super::ring_item::RingItem;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Similar to AsadStack, we have EvtStacks for the FRIBDAQ data.
///
/// FRIBDAQ .evt files are split every 2.0GB for legacy reasons. The stack is the
/// collection of all files associated with a given run in the FRIBDAQ system.
#[allow(dead_code)]
#[derive(Debug)]
pub struct EvtStack {
    pub file_stack: VecDeque<PathBuf>,
    active_file: EvtFile,
    pub total_stack_size_bytes: u64,
    is_ended: bool,
    parent_path: PathBuf,
}

impl EvtStack {
    /// Create a new EvtStack for a given FRIBDAQ run directory
    pub fn new(path: &Path) -> Result<Self, EvtStackError> {
        let (mut stack, bytes) = Self::get_file_stack(path)?;
        if let Some(file_path) = stack.pop_front() {
            Ok(EvtStack {
                file_stack: stack,
                active_file: EvtFile::new(&file_path)?,
                total_stack_size_bytes: bytes,
                is_ended: false,
                parent_path: PathBuf::from(path),
            })
        } else {
            Err(EvtStackError::NoMatchingFiles)
        }
    }

    /// Get the next ring item in the file stack
    ///
    /// Returns a `Result<Option<RingItem>>`. The Option is None if the stack has
    /// no more data.
    pub fn get_next_ring_item(&mut self) -> Result<Option<RingItem>, EvtStackError> {
        loop {
            if self.is_ended {
                return Ok(None);
            }

            match self.active_file.get_next_item() {
                Ok(ring) => return Ok(Some(ring)),
                Err(EvtFileError::EndOfFile) => {
                    self.move_to_next_file()?;
                }
                Err(e) => return Err(EvtStackError::FileError(e)),
            };
        }
    }

    /// Get all of the associated .evt files and put them in the stack
    fn get_file_stack(parent_path: &Path) -> Result<(VecDeque<PathBuf>, u64), EvtStackError> {
        let mut file_list: Vec<PathBuf> = Vec::new();
        let start_pattern = "run-";
        let end_pattern = ".evt";
        for item in parent_path.read_dir()? {
            let item_path = item?.path();
            let item_path_str = item_path.to_str().unwrap();
            if item_path_str.contains(start_pattern) && item_path_str.contains(end_pattern) {
                file_list.push(item_path);
            }
        }

        if file_list.is_empty() {
            return Err(EvtStackError::NoMatchingFiles);
        }

        let total_stack_size_bytes = file_list
            .iter()
            .fold(0, |sum, path| sum + path.metadata().unwrap().len());

        file_list.sort(); // Can sort standard. The only change should be the number at the tail.
        let stack = file_list.into();

        Ok((stack, total_stack_size_bytes))
    }

    ///Move to the next file in the stack
    fn move_to_next_file(&mut self) -> Result<(), EvtStackError> {
        loop {
            if let Some(next_file_path) = self.file_stack.pop_front() {
                let next_file = EvtFile::new(&next_file_path)?;
                if !next_file.is_eof() {
                    self.active_file = next_file;
                    return Ok(());
                }
            } else {
                self.is_ended = true;
                return Ok(());
            }
        }
    }
}
