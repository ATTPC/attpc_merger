// This module looks weird and feels weird, but it is an attempt to maintain the insanity
// of the original AT-TPC mapping style for the pad plane. General concept:
// [cobo, asad, aget, channel] -> pad
// However, the original format attempts to keep all information propagated to the downstream
// data (including the cobo, asad, aget, channel data) for safety(?) reasons. So we actually
// map
// [cobo, asad, aget, channel] -> HardwareID(cobo, asad, aget, channel, pad)
// which is overkill but whatever.
// HardwareID also implements Hash, so that we can use it as a key later. This hash
// is simply the pad number, which is of course unique by definition.
//
// If this feels ultra repetative and overcomplicated... that's because it is ultra
// repetative and overcomplicated
use std::fs::File;
use std::io::Read;
use std::path::Path;

use fxhash::FxHashMap;

use crate::hardware_id::{Detector, SiliconID};

use super::error::GetChannelMapError;
use super::hardware_id::{generate_uuid, HardwareID};

const MIN_ENTRIES_PER_LINE: usize = 4; //Min number of elements (cobo, asad, aget, ch)
const PAD_ENTRIES_PER_LINE: usize = 5; //Number of elements in a single row in the CSV file
const SI_ENTRIES_PER_LINE: usize = 7; //Number of elements in a single row in the CSV file

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

/// GetChannelMap contains the mapping of the individual hardware identifiers (CoBo ID, AsAd ID, AGET ID, AGET channel) to AT-TPC pad number.
///
/// This can change from experiment to experiment, so GetChannelMap reads in a CSV file where each row contains 5 elements. The first four are the
/// hardware identifiers (in the order listed previously) and the fifth is the pad number.
#[derive(Debug, Clone, Default)]
pub struct GetChannelMap {
    map: FxHashMap<u64, HardwareID>,
}

impl GetChannelMap {
    /// Create a new GetChannelMap
    /// If the path is None, we load the default that is bundled with the merger
    pub fn new(path: Option<&Path>) -> Result<Self, GetChannelMapError> {
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
        let mut uuid: u64;
        let mut hw_id: HardwareID;
        let mut det_info: Detector;

        let mut pm = GetChannelMap::default();

        let mut lines = contents.lines();
        lines.next(); // Skip the header
        for line in lines {
            let entries: Vec<&str> = line.split_terminator(",").collect();
            if entries.len() < PAD_ENTRIES_PER_LINE {
                return Err(GetChannelMapError::BadFileFormat);
            }

            cb_id = entries[0].parse()?;
            ad_id = entries[1].parse()?;
            ag_id = entries[2].parse()?;
            ch_id = entries[3].parse()?;

            if entries.len() == PAD_ENTRIES_PER_LINE {
                det_info = Detector::Pad(entries[4].parse()?);
            } else if entries.len() == SI_ENTRIES_PER_LINE {
                det_info =
                    Detector::Silicon(SiliconID::new(entries[4], entries[5], entries[6].parse()?)?);
            } else {
                return Err(GetChannelMapError::BadFileFormat);
            }

            uuid = generate_uuid(&cb_id, &ad_id, &ag_id, &ch_id);
            hw_id = HardwareID::new(&cb_id, &ad_id, &ag_id, &ch_id, &det_info);
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
        let map = match GetChannelMap::new(None) {
            Ok(m) => m,
            Err(_) => {
                panic!();
            }
        };
        let cobo_id: u8 = 7;
        let asad_id: u8 = 2;
        let aget_id: u8 = 1;
        let channel: u8 = 10;
        let pad = Detector::Pad(9908);
        let expected_id = HardwareID::new(&cobo_id, &asad_id, &aget_id, &channel, &pad);
        let given_id = match map.get_hardware_id(&cobo_id, &asad_id, &aget_id, &channel) {
            Some(id) => id,
            None => panic!(),
        };
        assert_eq!(expected_id, *given_id);
    }
}
