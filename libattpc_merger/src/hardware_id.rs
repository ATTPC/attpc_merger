use super::error::SiError;
use std::hash::Hash;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiDetector {
    UpstreamFront,
    UpstreamBack,
    DownstreamFront,
    DownstreamBack,
}

impl FromStr for SiDetector {
    type Err = SiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "upstream_front" {
            Ok(Self::UpstreamFront)
        } else if s == "upstream_back" {
            Ok(Self::UpstreamBack)
        } else if s == "downstream_front" {
            Ok(Self::DownstreamFront)
        } else if s == "downstream_back" {
            Ok(Self::DownstreamBack)
        } else {
            Err(SiError::Detector(s.to_string()))
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SiliconID {
    pub kind: SiDetector,
    pub channel: usize,
}

impl SiliconID {
    pub fn new(kind: &str, channel: usize) -> Result<Self, SiError> {
        Ok(Self {
            kind: SiDetector::from_str(kind)?,
            channel,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Detector {
    Silicon(SiliconID),
    Pad(usize),
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

impl Hash for HardwareID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.detector {
            Detector::Pad(p) => p.hash(state),
            Detector::Silicon(s) => s.channel.hash(state),
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
