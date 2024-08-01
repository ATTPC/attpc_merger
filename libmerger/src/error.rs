use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

use super::constants::*;

/*
   GrawData errors
*/
#[derive(Debug, Clone)]
pub enum GrawDataError {
    BadAgetID(u8),
    BadChannel(u8),
    BadTimeBucket(u16),
}

impl Display for GrawDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrawDataError::BadAgetID(id) => write!(f, "Invalid aget ID {} found in GrawData!", id),
            GrawDataError::BadChannel(chan) => {
                write!(f, "Invalid channel {} found in GrawData!", chan)
            }
            GrawDataError::BadTimeBucket(bucket) => {
                write!(f, "Invalid time bucket {} found in GrawData!", bucket)
            }
        }
    }
}

impl Error for GrawDataError {}

/*
   GrawFrame errors
*/
#[derive(Debug)]
pub enum GrawFrameError {
    IOError(std::io::Error),
    IncorrectMetaType(u8),
    IncorrectFrameSize(u32, u32),
    IncorrectFrameType(u16),
    IncorrectHeaderSize(u16),
    IncorrectItemSize(u16),
    BadDatum(GrawDataError),
}

impl From<std::io::Error> for GrawFrameError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<GrawDataError> for GrawFrameError {
    fn from(value: GrawDataError) -> Self {
        GrawFrameError::BadDatum(value)
    }
}

impl Display for GrawFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrawFrameError::IOError(e) => write!(f, "Error parsing buffer into GrawFrame: {}", e),
            GrawFrameError::IncorrectMetaType(t) => write!(
                f,
                "Incorrect meta type found for GrawFrame! Found: {} Expected: {}",
                t, EXPECTED_META_TYPE
            ),
            GrawFrameError::IncorrectFrameSize(s, cs) => write!(
                f,
                "Incorrect frame size found for GrawFrame! Found: {}, Expected: {}",
                s, cs
            ),
            GrawFrameError::IncorrectFrameType(t) => write!(
                f,
                "Incorrect frame type found for GrawFrame! Found: {}, Expected: {} or {}",
                t, EXPECTED_FRAME_TYPE_FULL, EXPECTED_FRAME_TYPE_PARTIAL
            ),
            GrawFrameError::IncorrectHeaderSize(s) => write!(
                f,
                "Incorrect header size found for GrawFrame! Found: {}, Expected: {}",
                s, EXPECTED_HEADER_SIZE
            ),
            GrawFrameError::IncorrectItemSize(s) => write!(
                f,
                "Incorrect item size found for GrawFrame! Found: {}, Expected: {} or {}",
                s, EXPECTED_ITEM_SIZE_FULL, EXPECTED_ITEM_SIZE_PARTIAL
            ),
            GrawFrameError::BadDatum(e) => write!(f, "Bad datum found in GrawFrame! Error: {}", e),
        }
    }
}

impl Error for GrawFrameError {}

/*
   GrawFile errors
*/

#[derive(Debug)]
pub enum GrawFileError {
    BadFrame(GrawFrameError),
    BadFilePath(PathBuf),
    EndOfFile,
    IOError(std::io::Error),
}

impl From<GrawFrameError> for GrawFileError {
    fn from(value: GrawFrameError) -> Self {
        GrawFileError::BadFrame(value)
    }
}

impl From<std::io::Error> for GrawFileError {
    fn from(value: std::io::Error) -> Self {
        GrawFileError::IOError(value)
    }
}

impl Display for GrawFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrawFileError::BadFrame(frame) => {
                write!(f, "Bad frame found when reading GrawFile! Error: {}", frame)
            }
            GrawFileError::BadFilePath(path) => write!(
                f,
                "File {} does not exist at GrawFile::new!",
                path.display()
            ),
            GrawFileError::EndOfFile => write!(f, "File reached end!"),
            GrawFileError::IOError(e) => write!(f, "GrawFile recieved an io error: {}!", e),
        }
    }
}

impl Error for GrawFileError {}

/*
   EvtItem errors
*/

#[derive(Debug)]
pub enum EvtItemError {
    IOError(std::io::Error),
    StackOrderError,
    ItemSizeError,
}

impl Display for EvtItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(e) => write!(f, "Error parsing buffer into Evt Item: {}", e),
            Self::StackOrderError => write!(f, "In Physics item, module stack was out of order!"),
            Self::ItemSizeError => write!(f, "RingItem buffer has insufficent size!"),
        }
    }
}

impl From<std::io::Error> for EvtItemError {
    fn from(value: std::io::Error) -> Self {
        return Self::IOError(value);
    }
}

impl Error for EvtItemError {}

/*
    EvtFile errors
*/

#[derive(Debug)]
pub enum EvtFileError {
    BadItem(EvtItemError),
    BadFilePath(PathBuf),
    EndOfFile,
    IOError(std::io::Error),
}

impl From<EvtItemError> for EvtFileError {
    fn from(value: EvtItemError) -> Self {
        EvtFileError::BadItem(value)
    }
}

impl From<std::io::Error> for EvtFileError {
    fn from(value: std::io::Error) -> Self {
        EvtFileError::IOError(value)
    }
}

impl Display for EvtFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvtFileError::BadItem(frame) => {
                write!(f, "Bad item found when reading evt File! Error: {}", frame)
            }
            EvtFileError::BadFilePath(path) => {
                write!(f, "File {} does not exist at EvtFile::new!", path.display())
            }
            EvtFileError::EndOfFile => write!(f, "File reached end!"),
            EvtFileError::IOError(e) => write!(f, "Evt File received an io error: {}!", e),
        }
    }
}

impl Error for EvtFileError {}

/*
    EvtStack errors
*/

#[derive(Debug)]
pub enum EvtStackError {
    IOError(std::io::Error),
    NoMatchingFiles,
    FileError(EvtFileError),
}

impl From<EvtFileError> for EvtStackError {
    fn from(value: EvtFileError) -> Self {
        Self::FileError(value)
    }
}

impl From<std::io::Error> for EvtStackError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl Display for EvtStackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(e) => write!(f, "EvtStack recieved an io error: {}", e),
            Self::FileError(e) => write!(f, "EvtStack recieved an EvtFileError: {}", e),
            Self::NoMatchingFiles => write!(
                f,
                "EvtStack did not find any matching files in the given directory!"
            ),
        }
    }
}

impl Error for EvtStackError {}

/*
   AsadStack errors
*/

#[derive(Debug)]
pub enum AsadStackError {
    IOError(std::io::Error),
    FileError(GrawFileError),
    NoMatchingFiles,
}

impl From<GrawFileError> for AsadStackError {
    fn from(value: GrawFileError) -> Self {
        Self::FileError(value)
    }
}

impl From<std::io::Error> for AsadStackError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl Display for AsadStackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(e) => write!(f, "AsadStack recieved an io error: {}", e),
            Self::FileError(e) => write!(f, "AsadStack recieved a file error: {}", e),
            Self::NoMatchingFiles => write!(f, "AsadStack couldn't find any matching files!"),
        }
    }
}

impl Error for AsadStackError {}

/*
   PadMap errors
*/

#[derive(Debug)]
pub enum PadMapError {
    IOError(std::io::Error),
    ParsingError(std::num::ParseIntError),
    BadFileFormat,
}

impl From<std::io::Error> for PadMapError {
    fn from(value: std::io::Error) -> Self {
        PadMapError::IOError(value)
    }
}

impl From<std::num::ParseIntError> for PadMapError {
    fn from(value: std::num::ParseIntError) -> Self {
        PadMapError::ParsingError(value)
    }
}

impl Display for PadMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PadMapError::IOError(e) => write!(f, "PadMap recieved an io error: {}", e),
            PadMapError::ParsingError(e) => write!(f, "PadMap error recieved a parsing error: {}", e),
            PadMapError::BadFileFormat => write!(f, "PadMap found a bad file format while reading the map file! Expected .csv without whitespaces")
        }
    }
}

impl Error for PadMapError {}

/*
   Event errors
*/
#[derive(Debug)]
#[allow(dead_code)]
pub enum EventError {
    InvalidHardware(u8, u8, u8, u8),
    MismatchedEventID(u32, u32),
}

impl Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::InvalidHardware(cb, ad, ag, ch) => write!(f, "Event found hardware which does not correspond to a valid pad! CoBo: {}, AsAd: {}, AGET: {}, Channel: {}", cb, ad, ag, ch),
            EventError::MismatchedEventID(given, exp) => write!(f, "Event was given a mismatched event id! Given: {}, Expected: {}", given, exp)
        }
    }
}

impl Error for EventError {}

/*
   Merger errors
*/

#[derive(Debug)]
pub enum MergerError {
    AsadError(AsadStackError),
    NoFilesError,
    IOError(std::io::Error),
    ConfigError(ConfigError),
}

impl From<AsadStackError> for MergerError {
    fn from(value: AsadStackError) -> Self {
        MergerError::AsadError(value)
    }
}

impl From<std::io::Error> for MergerError {
    fn from(value: std::io::Error) -> Self {
        MergerError::IOError(value)
    }
}

impl From<ConfigError> for MergerError {
    fn from(value: ConfigError) -> Self {
        MergerError::ConfigError(value)
    }
}

impl Display for MergerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergerError::AsadError(e) => {
                write!(f, "A stack error occurred while merging! Error: {}", e)
            }
            MergerError::NoFilesError => {
                write!(f, "Merger could not find any files with .graw extension!")
            }
            MergerError::IOError(e) => write!(f, "The merger recieved an io error: {}", e),
            MergerError::ConfigError(e) => {
                write!(f, "The merger encountered a config error: {}", e)
            }
        }
    }
}

impl Error for MergerError {}

/*
   EventBuilder errors
*/

#[derive(Debug)]
pub enum EventBuilderError {
    EventOutOfOrder(u32, u32),
    EventError(EventError),
}

impl From<EventError> for EventBuilderError {
    fn from(value: EventError) -> Self {
        Self::EventError(value)
    }
}

impl Display for EventBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventOutOfOrder(frame, event) => write!(f, "The event builder recieved a frame that is out of order -- frame event id: {} event builder event id: {}", frame, event),
            Self::EventError(val) => write!(f, "The EventBuilder recieved an event error: {}", val)
        }
    }
}

impl Error for EventBuilderError {}

/*
   Config errors
*/
#[derive(Debug)]
pub enum ConfigError {
    BadFilePath(PathBuf),
    IOError(std::io::Error),
    ParsingError(serde_yaml::Error),
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::IOError(value)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(value: serde_yaml::Error) -> Self {
        ConfigError::ParsingError(value)
    }
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadFilePath(path) => {
                write!(f, "File {} given to Config does not exist!", path.display())
            }
            Self::IOError(e) => write!(f, "Config received an io error: {}", e),
            Self::ParsingError(e) => write!(f, "Config received a parsing error: {}", e),
        }
    }
}

impl Error for ConfigError {}

#[derive(Debug)]
pub enum ProcessorError {
    EVBError(EventBuilderError),
    MergerError(MergerError),
    HDFError(hdf5::Error),
    ConfigError(ConfigError),
    MapError(PadMapError),
    EvtError(EvtStackError),
    BadRingConversion(EvtItemError),
}

impl From<MergerError> for ProcessorError {
    fn from(value: MergerError) -> Self {
        Self::MergerError(value)
    }
}

impl From<EventBuilderError> for ProcessorError {
    fn from(value: EventBuilderError) -> Self {
        Self::EVBError(value)
    }
}

impl From<hdf5::Error> for ProcessorError {
    fn from(value: hdf5::Error) -> Self {
        Self::HDFError(value)
    }
}

impl From<ConfigError> for ProcessorError {
    fn from(value: ConfigError) -> Self {
        Self::ConfigError(value)
    }
}

impl From<PadMapError> for ProcessorError {
    fn from(value: PadMapError) -> Self {
        Self::MapError(value)
    }
}

impl From<EvtStackError> for ProcessorError {
    fn from(value: EvtStackError) -> Self {
        Self::EvtError(value)
    }
}

impl From<EvtItemError> for ProcessorError {
    fn from(value: EvtItemError) -> Self {
        Self::BadRingConversion(value)
    }
}

impl Display for ProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EVBError(e) => write!(f, "Processor failed at Event Builder with error: {}", e),
            Self::MergerError(e) => write!(f, "Processor failed at Merger with error: {}", e),
            Self::HDFError(e) => write!(f, "Processor failed at HDFWriter with error: {}", e),
            Self::ConfigError(e) => write!(f, "Processor failed due to Configuration error: {}", e),
            Self::MapError(e) => write!(f, "Processor failed due to PadMap error: {}", e),
            Self::EvtError(e) => write!(f, "Processor failed due to evt stack error: {}", e),
            Self::BadRingConversion(e) => {
                write!(f, "Processor failed due to bad ring item conversion: {}", e)
            }
        }
    }
}

impl Error for ProcessorError {}
