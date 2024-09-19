use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

use super::error::EvtFileError;
use super::ring_item::RingItem;

/// Representation .evt files contain the data recorded by the FRIB DAQ system.
///
/// The data is atomic in RingItems that contain various types of data.
/// These RingItems can then be cast to functional types which parse the binary buffer
/// and allow the data to be accessed.
#[allow(dead_code)]
#[derive(Debug)]
pub struct EvtFile {
    file_handle: File,
    file_path: PathBuf,
    size_bytes: u64,
    is_eof: bool,
    is_open: bool,
}

impl EvtFile {
    /// Open a evt file in read-only mode.
    pub fn new(path: &Path) -> Result<Self, EvtFileError> {
        if !path.exists() {
            return Err(EvtFileError::BadFilePath(path.to_path_buf()));
        }

        let file_path = path.to_path_buf();
        let file_handle = File::open(path)?;
        let size_bytes = file_handle.metadata()?.len();

        Ok(EvtFile {
            file_handle,
            file_path,
            size_bytes,
            is_eof: false,
            is_open: true,
        })
    }

    /// Check if the file is still alive
    pub fn is_eof(&self) -> bool {
        self.is_eof
    }

    /// Retrieve the next RingItem from the buffer.
    ///
    /// Returns a `Result<RingItem>`. The RingItem can then be cast to
    /// the appropriate usable type.
    pub fn get_next_item(&mut self) -> Result<RingItem, EvtFileError> {
        //First need to query the size of the next ring item.
        let current_position: u64 = self.file_handle.stream_position()?;
        let item_size = match self.file_handle.read_u32::<LittleEndian>() {
            Ok(val) => val as usize,
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    self.is_eof = true;
                    return Err(EvtFileError::EndOfFile);
                }
                _ => {
                    return Err(EvtFileError::IOError(e));
                }
            },
        };

        self.file_handle.seek(SeekFrom::Start(current_position))?; // Go back to start of item (size is self contained)
        let mut buffer: Vec<u8> = vec![0; item_size]; // set size of bytes vector
        match self.file_handle.read_exact(&mut buffer) {
            // try to read ring item
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    self.is_eof = true;
                    Err(EvtFileError::EndOfFile)
                }
                _ => Err(EvtFileError::IOError(e)),
            },
            Ok(()) => Ok(RingItem::try_from(buffer)?),
        }
    }
}
