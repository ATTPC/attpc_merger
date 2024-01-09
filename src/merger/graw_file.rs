use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use super::constants::*;
use super::error::GrawFileError;
use super::graw_frame::{FrameMetadata, GrawFrame, GrawFrameHeader};

/// A .graw file is a raw data file produced by the AGET electronics system.
///
/// Each graw file is produced by a single AsAd board. Each AsAd board houses 4
/// AGET digitizer components. 4 AsAd's are managed by a single CoBo.
///
/// The functional purpose of the GrawFile is to provide an interface to the underlying binary data,
/// by providing methods which query the metadata (event data) of the next GrawFrame
/// (the functional data unit of a GrawFile) as well as retrieving the next GrawFrame.
#[allow(dead_code)]
#[derive(Debug)]
pub struct GrawFile {
    file_handle: File,
    file_path: PathBuf,
    size_bytes: u64,
    next_frame_metadata: FrameMetadata, // Store this to reduce read calls
    is_eof: bool,
    is_open: bool,
}

impl GrawFile {
    /// Open a graw file in read-only mode.
    pub fn new(path: &Path) -> Result<Self, GrawFileError> {
        if !path.exists() {
            return Err(GrawFileError::BadFilePath(path.to_path_buf()));
        }

        let file_path = path.to_path_buf();
        let file = File::open(path)?;
        let size_bytes = file.metadata()?.len();
        let handle = file;

        Ok(GrawFile {
            file_handle: handle,
            file_path,
            size_bytes: size_bytes,
            next_frame_metadata: FrameMetadata::default(),
            is_eof: false,
            is_open: true,
        })
    }

    /// Retrieve the next GrawFrame from the file
    pub fn get_next_frame(&mut self) -> Result<GrawFrame, GrawFileError> {
        let next_header = self.get_next_frame_header()?;
        let frame_read_size: usize = (next_header.frame_size * SIZE_UNIT) as usize;
        let mut frame_word: Vec<u8> = vec![0; frame_read_size];

        //Clear metadata
        self.next_frame_metadata = FrameMetadata::default();

        //Check to see if we reach end of file... shouldn't happen here tho
        match self.file_handle.read_exact(&mut frame_word) {
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    self.is_eof = true;
                    return Err(GrawFileError::EndOfFile);
                }
                _ => {
                    return Err(GrawFileError::IOError(e));
                }
            },
            Ok(()) => {
                return Ok(GrawFrame::try_from(frame_word)?);
            }
        }
    }

    /// Retrieve the metadata of the next frame. Note that this does not affect the buffer position
    pub fn get_next_frame_metadata(&mut self) -> Result<FrameMetadata, GrawFileError> {
        if self.next_frame_metadata == FrameMetadata::default() {
            self.next_frame_metadata = FrameMetadata::from(self.get_next_frame_header()?);
        }
        Ok(self.next_frame_metadata.clone())
    }

    /// Check to see if the file has ended
    pub fn is_eof(&self) -> &bool {
        &self.is_eof
    }

    /// Check if the file was succesfully opened
    pub fn is_open(&self) -> &bool {
        &self.is_open
    }

    /// Get the associated path on disk
    pub fn get_filename(&self) -> &Path {
        &self.file_path
    }

    /// Get the size of the file in bytes
    pub fn get_size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Peek at the header of the next frame to extract sizing information or metadata
    ///
    /// This resets the file stream to the position at the start of the header, as the read of the frame includes
    /// reading the header
    fn get_next_frame_header(&mut self) -> Result<GrawFrameHeader, GrawFileError> {
        let read_size: usize = (EXPECTED_HEADER_SIZE as u32 * SIZE_UNIT) as usize;
        let current_position = self.file_handle.stream_position()?;
        let mut header_word: Vec<u8> = vec![0; read_size];
        //Check to see if we reach end of file
        match self.file_handle.read_exact(&mut header_word) {
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    self.is_eof = true;
                    return Err(GrawFileError::EndOfFile);
                }
                _ => {
                    return Err(GrawFileError::IOError(e));
                }
            },
            Ok(_) => (),
        }
        let header = GrawFrameHeader::read_from_buffer(&mut Cursor::new(header_word))?;
        //Return to the start of the header
        self.file_handle
            .seek(std::io::SeekFrom::Start(current_position))?;
        Ok(header)
    }
}
