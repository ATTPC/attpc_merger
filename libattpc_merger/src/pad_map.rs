use std::fs::File;
use std::hash::Hash;
use std::io::Read;
use std::path::Path;

use fxhash::FxHashMap;

use super::error::PadMapError;

const ENTRIES_PER_LINE: usize = 5; //Number of elements in a single row in the CSV file

/// Load the default map for windows
#[cfg(target_family = "windows")]
fn load_default_map() -> String {
    String::from(include_str!("data\\default_pad_map.csv"))
}

/// Load the default map for macos and linux
#[cfg(target_family = "unix")]
fn load_default_map() -> String {
    String::from(include_str!("data/default_pad_map.csv"))
}

/// HardwareID is a hashable wrapper around the full hardware address (including the pad number).
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct HardwareID {
    pub cobo_id: usize,
    pub asad_id: usize,
    pub aget_id: usize,
    pub channel: usize,
    pub pad_id: usize,
}

impl HardwareID {
    /// Construct a new hardware ID
    pub fn new(cobo_id: &u8, asad_id: &u8, aget_id: &u8, channel: &u8, pad_id: &u64) -> Self {
        HardwareID {
            cobo_id: *cobo_id as usize,
            asad_id: *asad_id as usize,
            aget_id: *aget_id as usize,
            channel: *channel as usize,
            pad_id: *pad_id as usize,
        }
    }
}

impl Hash for HardwareID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pad_id.hash(state) //Just use the pad number as it is unqiue by definition
    }
}

/// Generate a unique id number for a given hardware location
fn generate_uuid(cobo_id: &u8, asad_id: &u8, aget_id: &u8, channel_id: &u8) -> u64 {
    (*channel_id as u64)
        + (*aget_id as u64) * 100
        + (*asad_id as u64) * 10_000
        + (*cobo_id as u64) * 1_000_000
}

/// PadMap contains the mapping of the individual hardware identifiers (CoBo ID, AsAd ID, AGET ID, AGET channel) to AT-TPC pad number.
///
/// This can change from experiment to experiment, so PadMap reads in a CSV file where each row contains 5 elements. The first four are the
/// hardware identifiers (in the order listed previously) and the fifth is the pad number.
#[derive(Debug, Clone, Default)]
pub struct PadMap {
    map: FxHashMap<u64, HardwareID>,
}

impl PadMap {
    /// Create a new PadMap
    /// If the path is None, we load the default that is bundled with the merger
    pub fn new(path: Option<&Path>) -> Result<Self, PadMapError> {
        let mut contents = String::new();
        if let Some(p) = path {
            let mut file = File::open(p)?;
            file.read_to_string(&mut contents)?;
        } else {
            contents = load_default_map();
        }

        let mut cb_id: u8;
        let mut ad_id: u8;
        let mut ag_id: u8;
        let mut ch_id: u8;
        let mut pd_id: u64;
        let mut uuid: u64;
        let mut hw_id: HardwareID;

        let mut pm = PadMap::default();

        let mut lines = contents.lines();
        lines.next(); // Skip the header
        for line in lines {
            let entries: Vec<&str> = line.split_terminator(",").collect();
            if entries.len() < ENTRIES_PER_LINE {
                return Err(PadMapError::BadFileFormat);
            }

            cb_id = entries[0].parse()?;
            ad_id = entries[1].parse()?;
            ag_id = entries[2].parse()?;
            ch_id = entries[3].parse()?;
            pd_id = entries[4].parse()?;

            uuid = generate_uuid(&cb_id, &ad_id, &ag_id, &ch_id);
            hw_id = HardwareID::new(&cb_id, &ad_id, &ag_id, &ch_id, &pd_id);
            pm.map.insert(uuid, hw_id);
        }

        Ok(pm)
    }

    /// Get the full HardwareID for a given set of hardware identifiers.
    ///
    /// If returns None the identifiers given do not exist in the map
    pub fn get_hardware_id(
        &self,
        cobo_id: &u8,
        asad_id: &u8,
        aget_id: &u8,
        channel_id: &u8,
    ) -> Option<&HardwareID> {
        let uuid = generate_uuid(cobo_id, asad_id, aget_id, channel_id);
        self.map.get(&uuid)
    }
}

//Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_map() {
        let map = match PadMap::new(None) {
            Ok(m) => m,
            Err(_) => {
                panic!();
            }
        };
        let cobo_id: u8 = 7;
        let asad_id: u8 = 2;
        let aget_id: u8 = 1;
        let channel: u8 = 10;
        let pad_id: u64 = 9908;
        let expected_id = HardwareID::new(&cobo_id, &asad_id, &aget_id, &channel, &pad_id);
        let given_id = match map.get_hardware_id(&cobo_id, &asad_id, &aget_id, &channel) {
            Some(id) => id,
            None => panic!(),
        };
        assert_eq!(expected_id, *given_id);
    }
}
