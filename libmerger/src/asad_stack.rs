use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::error::{AsadStackError, GrawFileError};
use super::graw_file::GrawFile;
use super::graw_frame::{FrameMetadata, GrawFrame};

/// AsadStack is representation of all of the files for a specific AsAd in a specific CoBo.
///
/// Data from the AT-TPC DAQ is written to files on a per AsAd-CoBo basis (each AsAd-CoBo gets its own file to write to).
/// These files are then split at approximately 1GB. This means that for a given run there can be many files for a given AsAd-CoBo.
/// AsadStack searches through the run directory structure and finds all files associated with with that specific AsAd-CoBo, sorts the files,
/// and then opens to earliest file as the active file. When that active file runs out of data, the stack moves to the next file in the queue,
/// and so on.
///
/// ## Why
/// This is more advantageous than simply opening all files, because we don't want to have to search through all possible files to find the earliest frame
/// when we dont have to. It can also save some memory/optimization by not having to buffer up all of the files around.
#[allow(dead_code)]
#[derive(Debug)]
pub struct AsadStack {
    active_file: GrawFile,
    file_stack: VecDeque<PathBuf>,
    cobo_number: i32,
    asad_number: i32,
    parent_path: PathBuf,
    total_stack_size_bytes: u64,
    is_ended: bool,
}

impl AsadStack {
    /// Create a new AsadStack for a given AsAd-CoBo combo in a given directory
    pub fn new(
        data_path: &Path,
        cobo_number: i32,
        asad_number: i32,
    ) -> Result<Self, AsadStackError> {
        let (mut file_stack, total_stack_size_bytes) =
            Self::get_file_stack(&data_path, &cobo_number, &asad_number)?;
        if let Some(path) = file_stack.pop_front() {
            //Activate the first file
            Ok(AsadStack {
                active_file: GrawFile::new(&path)?,
                file_stack,
                cobo_number,
                asad_number,
                parent_path: data_path.into(),
                total_stack_size_bytes,
                is_ended: false,
            })
        } else {
            Err(AsadStackError::NoMatchingFiles)
        }
    }

    /// Query the active file for the next frame's metadata.
    ///
    /// If there is nothing left to read, the stack attempts to move to the next file.
    /// Returns a `Result<Option<FrameMetadata>>`. If the Option is None, the stack has run out of data.
    ///
    /// # Important
    /// The metadata for the next frame should *always* be queried before attempting to retrieve the next frame.
    /// The get_next_frame will not attempt to move to the next file in the stack and will simply return an error if there is
    /// no more data in the active file.
    pub fn get_next_frame_metadata(&mut self) -> Result<Option<FrameMetadata>, AsadStackError> {
        loop {
            if self.is_ended {
                return Ok(None);
            }
            match self.active_file.get_next_frame_metadata() {
                Ok(meta) => return Ok(Some(meta)),
                Err(GrawFileError::EndOfFile) => {
                    self.move_to_next_file()?;
                    continue;
                }
                Err(e) => return Err(AsadStackError::FileError(e)),
            }
        }
    }

    /// Get the next GrawFrame from the active file.
    ///
    /// # Important
    /// The metadata for the next frame should *always* be queried before attempting to retrieve the next frame.
    /// The get_next_frame will not attempt to move to the next file in the stack and will simply return an error if there is
    /// no more data in the active file.
    pub fn get_next_frame(&mut self) -> Result<GrawFrame, AsadStackError> {
        Ok(self.active_file.get_next_frame()?)
    }

    /// The total size of the stack data in bytes
    pub fn get_stack_size_bytes(&self) -> &u64 {
        &self.total_stack_size_bytes
    }

    /// Get the CoBo id number for this stack
    pub fn get_cobo_number(&self) -> &i32 {
        &self.cobo_number
    }

    /// Get the AsAd id number for this stack
    pub fn get_asad_number(&self) -> &i32 {
        &self.asad_number
    }

    /// Get an immutable reference to the underlying file stack
    pub fn get_file_stack_ref(&self) -> &VecDeque<PathBuf> {
        &self.file_stack
    }

    /// Get an immutable reference to the current active file
    pub fn get_active_file(&self) -> &GrawFile {
        &self.active_file
    }

    /// Returns true if there is still data to be read from this stack. Returns false if the stack is finished.
    pub fn is_not_ended(&self) -> bool {
        !self.is_ended
    }

    /// Load the file stack
    ///
    /// Search the associated directory for the appropriate .graw files
    fn get_file_stack(
        parent_path: &Path,
        cobo_number: &i32,
        asad_number: &i32,
    ) -> Result<(VecDeque<PathBuf>, u64), AsadStackError> {
        let stack: VecDeque<PathBuf>;
        let mut file_list: Vec<PathBuf> = Vec::new();
        let start_pattern = format!("CoBo{}_AsAd{}", *cobo_number, *asad_number);
        let end_pattern = ".graw";
        for item in parent_path.read_dir()? {
            let item_path = item?.path();
            let item_path_str = item_path.to_str().unwrap();
            if item_path_str.contains(&start_pattern) && item_path_str.contains(&end_pattern) {
                file_list.push(item_path);
            }
        }

        if file_list.len() == 0 {
            return Err(AsadStackError::NoMatchingFiles);
        }

        let total_stack_size_bytes = file_list
            .iter()
            .fold(0, |sum, path| sum + path.metadata().unwrap().len());

        file_list.sort(); // Can sort standard. The only change should be in the number at the tail.
        stack = file_list.into();

        return Ok((stack, total_stack_size_bytes));
    }

    /// Move to the next file in the stack
    ///
    /// If there are no more files in the stack, the is_ended flag is set
    fn move_to_next_file(&mut self) -> Result<(), AsadStackError> {
        loop {
            if let Some(next_file_path) = self.file_stack.pop_front() {
                let next_file = GrawFile::new(&next_file_path)?;
                if *next_file.is_open() && !(*next_file.is_eof()) {
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
