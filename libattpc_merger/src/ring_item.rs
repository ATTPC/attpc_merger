use super::error::EvtItemError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::{fmt::UpperHex, io::{Cursor, Read}};

//These are the literal values for the different ring item type fields
const BEGIN_RUN_VAL: u8 = 1;
const END_RUN_VAL: u8 = 2;
const DUMMY_VAL: u8 = 12;
const SCALERS_VAL: u8 = 20;
const PHYSICS_VAL: u8 = 30;
const COUNTER_VAL: u8 = 31;

//Some Ring constants
const RING_HEADER_PRESENT: u8 = 20;
const HEADER_PRESENT_INDEX: usize = 28;
const NO_HEADER_INDEX: usize = 12;

/// RingType is an enum representing the type of data stored within a FRIBDAQ ring.
///
/// This allows for casting a generic RingItem to its functional type.
#[derive(Debug, Clone)]
pub enum RingType {
    BeginRun,
    EndRun,
    Dummy,
    Scalers,
    Physics,
    Counter,
    Invalid,
}

/// Convert the primitive byte to the RingType class.
impl From<u8> for RingType {
    fn from(value: u8) -> Self {
        match value {
            BEGIN_RUN_VAL => RingType::BeginRun,
            END_RUN_VAL => RingType::EndRun,
            DUMMY_VAL => RingType::Dummy,
            SCALERS_VAL => RingType::Scalers,
            PHYSICS_VAL => RingType::Physics,
            COUNTER_VAL => RingType::Counter,
            _ => RingType::Invalid,
        }
    }
}

/// RingItem is the base object of FRIBDAQ data.
///
/// A RingItem contains a buffer of bytes, a size, and a RingType
/// which can be used to cast the RingItem to its functional type.
#[derive(Debug, Clone)]
pub struct RingItem {
    pub size: usize,
    pub bytes: Vec<u8>,
    pub ring_type: RingType,
}

/// Convert the raw byte buffer to a RingItem.
impl TryFrom<Vec<u8>> for RingItem {
    type Error = EvtItemError;
    fn try_from(buffer: Vec<u8>) -> Result<Self, Self::Error> {
        let rt_data: u8;
        {
            let type_data = buffer.get(4);
            match type_data {
                Some(data) => rt_data = *data,
                None => return Err(EvtItemError::ItemSizeError),
            };
        }
        //RingItems can optionally have a header. We trim this header
        let item_data_buffer: Vec<u8>;
        if buffer[8] == RING_HEADER_PRESENT && buffer.len() >= HEADER_PRESENT_INDEX {
            item_data_buffer = buffer[HEADER_PRESENT_INDEX..].to_vec();
        } else if buffer.len() >= NO_HEADER_INDEX {
            item_data_buffer = buffer[NO_HEADER_INDEX..].to_vec();
        } else {
            return Err(EvtItemError::ItemSizeError);
        }
        Ok(Self {
            size: buffer.len(),
            bytes: item_data_buffer,
            ring_type: RingType::from(rt_data),
        })
    }
}

impl Default for RingItem {
    fn default() -> Self {
        Self {
            size: 0,
            bytes: vec![],
            ring_type: RingType::Invalid,
        }
    }
}

impl RingItem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove VMUSB buffer boundaries from the RingItem data buffer.
    ///
    /// Somtimes physics item data is large enough to run over the VMUSB boundary
    /// which leaves an empty word in the item data.
    /// # Note
    /// Only use this function for PhysicsItems
    pub fn remove_boundaries(&mut self) {
        let mut wlength: u16;
        let mut buf: [u8; 2] = [0, 0];
        let mut ind: usize = 0;
        while ind < self.bytes.len() {
            buf.copy_from_slice(&self.bytes[ind..ind + 2]);
            wlength = u16::from_le_bytes(buf) & 0xfff; // buffer length
            self.bytes.remove(ind);
            self.bytes.remove(ind); // 2 bytes to remove
            ind += usize::from(wlength * 2); // next boundary
        }
    }
}

// Below are the various explicit ring item types. RingItems can be cast into these objects using
// try_from semantics.

/// RingItem which contains the run number, the start time, and the run title
#[derive(Debug, Clone, Default)]
pub struct BeginRunItem {
    pub run: u32,
    pub start: u32,
    pub title: String,
}

/// Cast a RingItem to a BeginRunItem
impl TryFrom<RingItem> for BeginRunItem {
    type Error = EvtItemError;
    fn try_from(ring: RingItem) -> Result<Self, EvtItemError> {
        let mut cursor = Cursor::new(ring.bytes);
        let mut info = BeginRunItem::new();
        info.run = cursor.read_u32::<LittleEndian>()?;
        cursor.set_position(cursor.position() + 4);
        info.start = cursor.read_u32::<LittleEndian>()?;
        cursor.set_position(cursor.position() + 4);
        cursor.read_to_string(&mut info.title)?;
        Ok(info)
    }
}

impl BeginRunItem {
    pub fn new() -> Self {
        Self::default()
    }
}

/// RingItem which contains the run stop time, and the ellapsed time.
#[derive(Debug, Clone, Default)]
pub struct EndRunItem {
    pub stop: u32,
    pub time: u32,
}

/// Cast a RingItem to an EndRunItem
impl TryFrom<RingItem> for EndRunItem {
    type Error = EvtItemError;
    fn try_from(ring: RingItem) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(ring.bytes);
        let mut info = EndRunItem::new();

        info.stop = cursor.read_u32::<LittleEndian>()?;
        info.time = cursor.read_u32::<LittleEndian>()?;
        Ok(info)
    }
}

impl EndRunItem {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Simple container for the begin and end run info for ease of use with HDF
#[derive(Debug, Clone, Default)]
pub struct RunInfo {
    pub begin: BeginRunItem,
    pub end: EndRunItem,
}

impl RunInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RunInfo {
    /// Get a formatted string of the BeginRunItem
    pub fn print_begin(&self) -> String {
        format!("Run Number: {} Title: {}", self.begin.run, self.begin.title)
    }

    /// Get a formatted string of the EndRunItem
    pub fn print_end(&self) -> String {
        format!(
            "Run Number: {} Elapsed Time: {}s",
            self.begin.run, self.end.time
        )
    }
}
/// A RingItem which contains the information from the FRIBDAQ scalers, or counters.
///
/// Scalers are composed of a header containing the timing of the scaler data
/// and a data vector that contains the scalers themselves (32 bits). The order of the scalers
/// is defined by FRIBDAQ.
#[derive(Debug, Clone, Default)]
pub struct ScalersItem {
    pub start_offset: u32,
    pub stop_offset: u32,
    pub timestamp: u32,
    pub incremental: u32,
    pub data: Vec<u32>,
}

/// Cast a RingItem to a ScalersItem
impl TryFrom<RingItem> for ScalersItem {
    type Error = EvtItemError;
    fn try_from(ring: RingItem) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(ring.bytes);
        let mut info = ScalersItem::new();
        info.start_offset = cursor.read_u32::<LittleEndian>()?;
        info.stop_offset = cursor.read_u32::<LittleEndian>()?;
        info.timestamp = cursor.read_u32::<LittleEndian>()?;
        let _dummy = cursor.read_u32::<LittleEndian>()?; // Dummy read
        let count = cursor.read_u32::<LittleEndian>()?; // This is where the number of scalers actually is
        info.incremental = cursor.read_u32::<LittleEndian>()?;
        info.data.resize(count as usize, 0);
        for value in info.data.iter_mut() {
            *value = cursor.read_u32::<LittleEndian>()?;
        }

        Ok(info)
    }
}

impl ScalersItem {
    pub fn new() -> Self {
        Self::default()
    }
}

/// A RingItem which contains the count of the number of physics items found by FRIBDAQ.
#[derive(Debug, Clone, Default)]
pub struct CounterItem {
    pub count: u64,
}

/// Cast a RingItem into a CounterItem
impl TryFrom<RingItem> for CounterItem {
    type Error = EvtItemError;
    fn try_from(ring: RingItem) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(ring.bytes);
        let mut info = Self::new();
        cursor.set_position(12);
        info.count = cursor.read_u64::<LittleEndian>()?;
        Ok(info)
    }
}

impl CounterItem {
    pub fn new() -> Self {
        Self::default()
    }
}

/// A RingItem which contains the data of the modules read by the VMEUSB controller stack in
/// FRIBDAQ. It is called Physics because this typically contains the data related to physical observables.
///
/// For now this an ad hoc list that only contains the modules present in the readout not a comprehensive list
/// of posibilities.
///
/// # Warning
/// If the VMEUSB stack is modified from the standard AT-TPC layout (the daqconfig.tcl script of FRIBDAQ),
/// the data will not be unpacked properly.
#[derive(Debug, Clone)]
pub struct PhysicsItem {
    pub event: u32,
    pub timestamp: u32,
    pub fadc1: SIS3300Item,
    pub fadc2: SIS3300Item,
    pub fadc3: SIS3300Item,
    pub fadc4: SIS3316Item,
    pub coinc: V977Item,
}

/// Cast a RingItem to a PhysicsItem
impl TryFrom<RingItem> for PhysicsItem {
    type Error = EvtItemError;
    fn try_from(ring: RingItem) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(ring.bytes);
        let mut info = PhysicsItem::new();
        info.event = cursor.read_u32::<LittleEndian>()?;
        info.timestamp = cursor.read_u32::<LittleEndian>()?;
        // Parse the stack. Order matters!
        // DB 2025-06-05 modified to loop on tags for more flexibility
        loop {
            let tag = match cursor.read_u16::<LittleEndian>() {
                Ok(tag) => tag,
                Err(_e) => break,
            };
            // println!("Tag found: {:X}", tag);
            
            if tag == 0x1903 {
                info.fadc1.extract_data(&mut cursor)?;
            } else if tag == 0x1904 {
                info.fadc2.extract_data(&mut cursor)?;
            } else if tag == 0x1905 {
                info.fadc3.extract_data(&mut cursor)?;
            } else if tag == 0x1906 {
                info.fadc4.extract_data(&mut cursor)?;
            } else if tag == 0x977 {
                info.coinc.extract_data(&mut cursor)?;
            } else if tag == 0xFFFF {
                cursor.set_position(cursor.position() - 2);
                break;
            }
            else {
                // If unknown tag, bail out
                println!("No match found for tag: {:X}", tag);
                cursor.set_position(cursor.position() - 2);
                break;
            }
        }
        // if cursor.read_u16::<LittleEndian>()? != 0x1903 {
        //     return Err(EvtItemError::StackOrderError);
        // }
        // info.fadc.extract_data(&mut cursor)?;
        // if cursor.read_u16::<LittleEndian>()? != 0x977 {
        //     return Err(EvtItemError::StackOrderError);
        // }
        // info.coinc.extract_data(&mut cursor)?;

        Ok(info)
    }
}

impl Default for PhysicsItem {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsItem {
    pub fn new() -> PhysicsItem {
        PhysicsItem {
            event: 0,
            timestamp: 0,
            fadc1: SIS3300Item::new(),
            fadc2: SIS3300Item::new(),
            fadc3: SIS3300Item::new(),
            fadc4: SIS3316Item::new(),
            coinc: V977Item::new(),
        }
    }
}

/// Item from Struck modules SIS3300 & SIS3301: 8 channel flash ADC (12 & 14 bits)
#[derive(Debug, Clone)]
pub struct SIS3300Item {
    pub traces: Vec<Vec<u16>>,
    pub samples: usize,
    pub channels: usize,
    pub hasdata: bool,
}

impl Default for SIS3300Item {
    fn default() -> Self {
        Self::new()
    }
}

impl SIS3300Item {
    pub fn new() -> SIS3300Item {
        SIS3300Item {
            traces: vec![vec![]; 8],
            samples: 0,
            channels: 0,
            hasdata: false,
        }
    }

    /// Extract the relevant data from the PhysicsItem buffer.
    ///
    /// This module is fairly nasty to parse. It contains a circular memory element for handling large
    /// data transfers. As such, the start index within the data is somewhat arbitrary.
    pub fn extract_data(
        &mut self,
        cursor: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<(), EvtItemError> {
        let group_enable_flags = cursor.read_u16::<LittleEndian>()?;
        let _daq_register = cursor.read_u32::<LittleEndian>()?; //Never used, but must be read

        //Important buffer elements
        let mut header: u16;
        let mut group_trigger: u32;
        let mut pointer: usize;
        let mut trailer: u16;

        //The module has four groups of channels
        for group in 0..4 {
            // if group_enable_flags & (1 >> group) == 0 {
            //     // skip if group is not enabled and fill array with zeros
            //     self.channels += 2;
            //     if self.samples > 0 {
            //         self.traces[group * 2] = vec![0; self.samples];
            //         self.traces[group * 2 + 1] = vec![0; self.samples];
            //     }
            //     continue;
            // }
            self.channels += 2; // channels are read in pairs
            header = cursor.read_u16::<LittleEndian>()?;
            if header != 0xfadc {
                spdlog::error!("Invalid SIS3300/1 header: {:#x}!", header);
                break;
            }
            group_trigger = cursor.read_u32::<LittleEndian>()?;
            self.samples = cursor.read_u32::<LittleEndian>()? as usize;
            self.traces[group * 2] = vec![0; self.samples];
            self.traces[group * 2 + 1] = vec![0; self.samples];
            pointer = (group_trigger & 0x1ffff) as usize; // write pointer (start location in the buffer)
            let starting_position = cursor.position(); // the original position of the cursor
                                                       //Handle a non-normal initial position in the buffer
            if ((group_trigger & 0x80000) != 0) && (pointer < self.samples - 1) {
                // if wrap around bit == 1
                let istart: usize = pointer + 1;
                let inc: usize = self.samples - pointer - 2;
                cursor.set_position(starting_position + ((istart * 4) as u64));
                for p in 0..inc + 1 {
                    self.traces[group * 2 + 1][p] = cursor.read_u16::<LittleEndian>()? & 0xfff;
                    self.traces[group * 2][p] = cursor.read_u16::<LittleEndian>()? & 0xfff;
                }
                //Wrap back around and read the remaining data
                let istop: usize = self.samples - inc - 1;
                cursor.set_position(starting_position);
                for p in 0..istop {
                    self.traces[group * 2 + 1][p + inc + 1] =
                        cursor.read_u16::<LittleEndian>()? & 0xfff;
                    self.traces[group * 2][p + inc + 1] =
                        cursor.read_u16::<LittleEndian>()? & 0xfff;
                }
            } else {
                for p in 0..self.samples {
                    self.traces[group * 2 + 1][p] = cursor.read_u16::<LittleEndian>()? & 0xfff;
                    self.traces[group * 2][p] = cursor.read_u16::<LittleEndian>()? & 0xfff;
                }
            }
            cursor.set_position(starting_position + ((self.samples * 4) as u64));
            trailer = cursor.read_u16::<LittleEndian>()?;
            if trailer != 0xffff {
                spdlog::error!("Invalid SIS3300 trailer: {:#x}!", trailer);
                break;
            }
            self.hasdata = true;
        }

        Ok(())
    }
}

/// Item from CAEN module V977: 16 bit coincidence register
///
/// A simple coicidence flag buffer
#[derive(Debug, Clone, Default)]
pub struct V977Item {
    pub coinc: u16,
}

impl V977Item {
    pub fn new() -> Self {
        Self::default()
    }

    /// Nothing too fancy. Read a single u16 from the PhysicsItem buffer
    pub fn extract_data(&mut self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), EvtItemError> {
        self.coinc = cursor.read_u16::<LittleEndian>()?;
        Ok(())
    }
}

/// Item from Struck modules SIS3316: 16 channel flash ADC (14 bits)
#[derive(Debug, Clone)]
pub struct SIS3316Item {
    pub traces: Vec<Vec<u16>>,
    pub samples: usize,
    pub channels: usize,
    pub valid: Vec<bool>,
    pub hasdata: bool,
}

impl Default for SIS3316Item {
    fn default() -> Self {
        Self::new()
    }
}

impl SIS3316Item {
    pub fn new() -> SIS3316Item {
        SIS3316Item {
            traces: vec![vec![]; 16],
            samples: 0,
            channels: 0,
            valid: vec![false; 16],
            hasdata: false,
        }
    }

    /// Extract the relevant data from the PhysicsItem buffer.
    pub fn extract_data(
        &mut self,
        cursor: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<(), EvtItemError> {
        let mut channel = ((cursor.read_u16::<LittleEndian>()?) >> 4 & 0xf) as usize;
        let mut _stamp1 = cursor.read_u32::<LittleEndian>()?;
        let mut _stamp2 = cursor.read_u16::<LittleEndian>()?;
        self.samples = (cursor.read_u16::<LittleEndian>()? * 2) as usize;
        let mut _status = cursor.read_u16::<LittleEndian>()?;
        let mut next: u16;

        loop {
            self.valid[channel] = true;
            self.channels += 1;
            self.traces[channel] = vec![0; self.samples + 1];
            self.traces[channel][0] = channel as u16; // Encode channel number as first datum
            for i in 0..self.samples {
                self.traces[channel][i + 1] = cursor.read_u16::<LittleEndian>()?;
            }
            next = cursor.read_u16::<LittleEndian>()?;
            cursor.set_position(cursor.position() - 2);
            if next == 0xffff {
                break;
            }
            channel = ((cursor.read_u16::<LittleEndian>()?) >> 4 & 0xf) as usize;
            _stamp1 = cursor.read_u32::<LittleEndian>()?;
            _stamp2 = cursor.read_u16::<LittleEndian>()?;
            self.samples = (cursor.read_u16::<LittleEndian>()? * 2) as usize;
            _status = cursor.read_u16::<LittleEndian>()?;
        }
        self.hasdata = true;

        Ok(())
    }
}
