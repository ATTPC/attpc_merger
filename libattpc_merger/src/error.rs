use std::path::PathBuf;
use thiserror::Error;

use super::constants::*;
use super::worker_status::WorkerStatus;

#[derive(Debug, Clone, Error)]
pub enum GrawDataError {
    #[error("Invalid aget ID {0} found in GrawData")]
    BadAgetID(u8),
    #[error("Invalid channel {0} found in GrawData")]
    BadChannel(u8),
    #[error("Invalid time bucket {0} found in GrawData")]
    BadTimeBucket(u16),
}

#[derive(Debug, Error)]
pub enum GrawFrameError {
    #[error("Failed to parse buffer into GrawFrame: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Incorrect meta type {0} found for GrawFrame; expected {exp}", exp=EXPECTED_META_TYPE)]
    IncorrectMetaType(u8),
    #[error("Incorrect frame size {0} found for GrawFrame; expected {1}")]
    IncorrectFrameSize(u32, u32),
    #[error("Incorrect frame type {0} found for GrawFrame; expected {exp1} or {exp2}", exp1=EXPECTED_FRAME_TYPE_FULL, exp2=EXPECTED_FRAME_TYPE_PARTIAL)]
    IncorrectFrameType(u16),
    #[error("Incorrect header size {0} found for GrawFrame; expected {size}", size=EXPECTED_HEADER_SIZE)]
    IncorrectHeaderSize(u16),
    #[error("Incorrect item size {0} found for GrawFrame; expected {size1} or {size2}", size1=EXPECTED_ITEM_SIZE_FULL, size2=EXPECTED_ITEM_SIZE_PARTIAL)]
    IncorrectItemSize(u16),
    #[error("Bad datum found in GrawFrame: {0}")]
    BadDatum(#[from] GrawDataError),
}

#[derive(Debug, Error)]
pub enum GrawFileError {
    #[error("Error when parsing GrawFrame from GrawFile: {0}")]
    BadFrame(#[from] GrawFrameError),
    #[error("Could not open GrawFile because file {0:?} does not exist")]
    BadFilePath(PathBuf),
    #[error("Reached end of GrawFile")]
    EndOfFile,
    #[error("GrawFile failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum EvtItemError {
    #[error("Error parsing buffer into a FRIBDAQ EvtItem: {0}")]
    IOError(#[from] std::io::Error),
    #[error("In FRIBDAQ PhysicsItem, the module stack was out of order and failed")]
    StackOrderError,
    #[error("In FRIBDAQ RingItem, the buffer has insufficent size and failed")]
    ItemSizeError,
}

#[derive(Debug, Error)]
pub enum EvtFileError {
    #[error("Reading item from FRIBDAQ EvtFile failed: {0}")]
    BadItem(#[from] EvtItemError),
    #[error("Could not open FRIBDAQ EvtFile because file {0:?} does not exist")]
    BadFilePath(PathBuf),
    #[error("FRIBDAQ EvtFile reached end-of-file")]
    EndOfFile,
    #[error("FRIBDAQ EvtFile recieved an IO error and failed: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum EvtStackError {
    #[error("FRIBDAQ EvtStack failed with IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("FRIBDAQ EvtStack did not find any matching files in the Evt directory")]
    NoMatchingFiles,
    #[error("EvtStack failed due to EvtFile error: {0}")]
    FileError(#[from] EvtFileError),
}

#[derive(Debug, Error)]
pub enum AsadStackError {
    #[error("AsAdStack failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("AsAdStack failed due to GrawFile error: {0}")]
    FileError(#[from] GrawFileError),
    #[error("AsAdStack could not find any matching GRAW files in the GRAW directory")]
    NoMatchingFiles,
}

#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("Found invalid detector keyword: {0}")]
    InvalidKeyword(String),
}

#[derive(Debug, Error)]
pub enum GetChannelMapError {
    #[error("GetChannelMap failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("GetChannelMap failed to parse an integer: {0}")]
    ParsingError(#[from] std::num::ParseIntError),
    #[error("GetChannelMap failed to parse a detector keyword: {0}")]
    BadDetKeyword(#[from] DetectorError),
    #[error("GetChannelMap was given a file with the incorrect format; most likely the number of columns is incorrect")]
    BadFileFormat,
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum EventError {
    #[error("Event received hardware which does not correspond to a valid channel -- CoBo: {0}, AsAd: {1}, AGET: {2}, Channel: {3}")]
    InvalidHardware(u8, u8, u8, u8),
    #[error("An Event was given data with a mismatched ID -- Given: {0} Expected: {1}")]
    MismatchedEventID(u32, u32),
}

#[derive(Debug, Error)]
pub enum MergerError {
    #[error("Merger failed due to AsAdStack error: {0}")]
    AsadError(#[from] AsadStackError),
    #[error("Merger failed because no GRAW files were found in the GRAW directory")]
    NoFilesError,
    #[error("Merger failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Merger failed due to configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

#[derive(Debug, Error)]
pub enum EventBuilderError {
    #[error("EventBuilder failed due a frame that was out of order -- frame event ID: {0} event builder event ID: {1}")]
    EventOutOfOrder(u32, u32),
    #[error("EventBuilder failed due to event error: {0}")]
    EventError(#[from] EventError),
}

#[derive(Debug, Error)]
pub enum HDF5WriterError {
    #[error("HDF5Writer failed due to HDF5 error: {0}")]
    HDF5Error(#[from] hdf5::Error),
    #[error("HDF5Writer failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("HDFWriter failed to convert to yaml: {0}")]
    ParsingError(#[from] serde_yaml::Error),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration as file {0:?} does not exist")]
    BadFilePath(PathBuf),
    #[error("Config failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Config failed to parse YAML: {0}")]
    ParsingError(#[from] serde_yaml::Error),
}

#[derive(Debug, Error)]
pub enum FileCopierError {
    #[error("FileCopier failed due to IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("FileCopier failed to find any matching files in the Evt directory")]
    NoMatchingFiles,
    #[error("FileCopier failed due to configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

#[derive(Debug, Error)]
pub enum FribBuilderError {
    #[error("FribBuilder failed due to evt_path is none")]
    NoneEvtPathError,
    #[error("FribBuilder failed due to EvtStack error: {0}")]
    EvtError(#[from] EvtStackError),
    #[error("FribBuilder failed due to EvtItem error: {0}")]
    BadRingConversion(#[from] EvtItemError),
    #[error("FribBuilder failed due to Config error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("FribBuilder failed due to HDF5Writer error: {0}")]
    HDFError(#[from] HDF5WriterError),
}

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Processor failed due to FileCopier error: {0}")]
    FileCopier(#[from] FileCopierError),
    #[error("Processor failed due to EventBuilder error: {0}")]
    EVBError(#[from] EventBuilderError),
    #[error("Processor failed due to Merger error: {0}")]
    MergerError(#[from] MergerError),
    #[error("Processor failed due to HDF5Writer error: {0}")]
    HDFError(#[from] HDF5WriterError),
    #[error("Processor failed due to Config error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Processor failed due to GetChannelMap error: {0}")]
    MapError(#[from] GetChannelMapError),
    #[error("Processor failed due to FribBuilder error: {0}")]
    FribBuilderError(#[from] FribBuilderError),
    #[error("Processor failed due to Send error: {0}")]
    SendError(#[from] std::sync::mpsc::SendError<WorkerStatus>),
    #[error("Processor failed due to IO error: {0}")]
    IoError(#[from] std::io::Error),
}
