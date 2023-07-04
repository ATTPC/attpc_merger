
//Data sizes and types
pub const EXPECTED_META_TYPE: u8 = 8;
pub const EXPECTED_HEADER_SIZE: u16 = 1;
pub const EXPECTED_ITEM_SIZE_PARTIAL: u16 = 4;
pub const EXPECTED_ITEM_SIZE_FULL: u16 = 2;
pub const EXPECTED_FRAME_TYPE_PARTIAL: u16 = 1;
pub const EXPECTED_FRAME_TYPE_FULL: u16 = 2;
pub const SIZE_UNIT: u32 = 256;
pub const SIZE_OF_BITSET: usize = 72;

//Electronics constants
pub const NUMBER_OF_COBOS: u8 = 10; //total
pub const NUMBER_OF_ASADS: u8 = 4; //per cobo
pub const NUMBER_OF_AGETS: u8 = 4; // per asad
pub const NUMBER_OF_CHANNELS: u8 = 68;
pub const NUMBER_OF_TIME_BUCKETS: u32 = 512;
