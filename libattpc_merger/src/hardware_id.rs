use super::error::DetectorError;

const PAD_STRING: &str = "pad";
const SI_UPFRONT_STRING: &str = "si_upstream_front";
const SI_UPBACK_STRING: &str = "si_upstream_back";
const SI_DOWNFRONT_STRING: &str = "si_downstream_front";
const SI_DOWNBACK_STRING: &str = "si_downstream_back";

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Detector {
    SiUpstreamFront(usize),
    SiUpstreamBack(usize),
    SiDownstreamFront(usize),
    SiDownstreamBack(usize),
    Pad(usize),
}

impl Detector {
    pub fn from_str_channel(det_str: &str, channel: usize) -> Result<Self, DetectorError> {
        if det_str == PAD_STRING {
            Ok(Self::Pad(channel))
        } else if det_str == SI_UPFRONT_STRING {
            Ok(Self::SiUpstreamFront(channel))
        } else if det_str == SI_UPBACK_STRING {
            Ok(Self::SiUpstreamBack(channel))
        } else if det_str == SI_DOWNFRONT_STRING {
            Ok(Self::SiDownstreamFront(channel))
        } else if det_str == SI_DOWNBACK_STRING {
            Ok(Self::SiDownstreamBack(channel))
        } else {
            Err(DetectorError::InvalidKeyword(det_str.to_string()))
        }
    }
}

// For pad plane

/// HardwareID is a hashable wrapper around the full hardware address (including the pad number).
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct HardwareID {
    pub cobo_id: usize,
    pub asad_id: usize,
    pub aget_id: usize,
    pub channel: usize,
    pub detector: Detector,
}

impl HardwareID {
    /// Construct a new hardware ID
    pub fn new(
        cobo_id: &u8,
        asad_id: &u8,
        aget_id: &u8,
        channel: &u8,
        detector: &Detector,
    ) -> Self {
        HardwareID {
            cobo_id: *cobo_id as usize,
            asad_id: *asad_id as usize,
            aget_id: *aget_id as usize,
            channel: *channel as usize,
            detector: detector.clone(),
        }
    }
}

/// Generate a unique id number for a given hardware location
pub fn generate_uuid(cobo_id: &u8, asad_id: &u8, aget_id: &u8, channel_id: &u8) -> u64 {
    (*channel_id as u64)
        + (*aget_id as u64) * 100
        + (*asad_id as u64) * 10_000
        + (*cobo_id as u64) * 1_000_000
}
