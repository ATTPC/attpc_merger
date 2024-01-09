use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::error::ConfigError;

/// Structure representing the application configuration. Contains pathing and run information
/// Configs are seralizable and deserializable to YAML using serde and serde_yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub graw_path: PathBuf,
    pub evt_path: PathBuf,
    pub hdf_path: PathBuf,
    pub pad_map_path: PathBuf,
    pub first_run_number: i32,
    pub last_run_number: i32,
    pub online: bool,
    pub experiment: String,
}

impl Config {
    #[allow(dead_code)]
    /// Generate a new Config object. All fields will be empty/invalid
    pub fn default() -> Self {
        Self {
            graw_path: PathBuf::from("None"),
            evt_path: PathBuf::from("None"),
            hdf_path: PathBuf::from("None"),
            pad_map_path: PathBuf::from("None"),
            first_run_number: 0,
            last_run_number: 0,
            online: false,
            experiment: String::from(""),
        }
    }

    /// Read the configuration in a YAML file
    /// Returns a Config if successful
    pub fn read_config_file(config_path: &Path) -> Result<Self, ConfigError> {
        if !config_path.exists() {
            return Err(ConfigError::BadFilePath(config_path.to_path_buf()));
        }

        let yaml_str = std::fs::read_to_string(config_path)?;

        Ok(serde_yaml::from_str::<Self>(&yaml_str)?)
    }

    /// Check if a specific run exists by evaluating the existance of *both* FRIBDAQ data and GET DAQ data
    pub fn does_run_exist(&self, run_number: i32) -> bool {
        let run_dir: PathBuf = self.graw_path.join(self.get_run_str(run_number));
        let evt_dir: PathBuf = self.evt_path.join(format!("run{}", run_number));
        if self.online {
            // Don't check run_dir if online
            return evt_dir.exists();
        } else {
            return run_dir.exists() && evt_dir.exists();
        }
    }

    /// Get the Path to a run file
    pub fn get_run_directory(&self, run_number: i32, cobo: &u8) -> Result<PathBuf, ConfigError> {
        let mut run_dir: PathBuf = self.graw_path.join(self.get_run_str(run_number));
        run_dir = run_dir.join(format!("mm{}", cobo));
        if run_dir.exists() {
            return Ok(run_dir);
        } else {
            return Err(ConfigError::BadFilePath(run_dir));
        }
    }

    /// Get the path to the online data, assuming the standard AT-TPC Server configuration
    pub fn get_online_directory(&self, run_number: i32, cobo: &u8) -> Result<PathBuf, ConfigError> {
        let mut online_dir: PathBuf = PathBuf::new().join(format!("/Volumes/mm{}", cobo));
        online_dir = online_dir.join(format!("{}", self.experiment));
        online_dir = online_dir.join(self.get_run_str(run_number));
        if online_dir.exists() {
            return Ok(online_dir);
        } else {
            return Err(ConfigError::BadFilePath(online_dir));
        }
    }

    /// Get the path to the FRIBDAQ directory, assuming the standard AT-TPC configuration
    pub fn get_evt_directory(&self, run_number: i32) -> Result<PathBuf, ConfigError> {
        let run_dir: PathBuf = self.evt_path.join(format!("run{}", run_number));
        if run_dir.exists() {
            return Ok(run_dir);
        } else {
            return Err(ConfigError::BadFilePath(run_dir));
        }
    }

    /// Get the path to the output hdf5 file
    pub fn get_hdf_file_name(&self, run_number: i32) -> Result<PathBuf, ConfigError> {
        let hdf_file_path: PathBuf = self
            .hdf_path
            .join(format!("{}.h5", self.get_run_str(run_number)));
        if self.hdf_path.exists() {
            return Ok(hdf_file_path);
        } else {
            return Err(ConfigError::BadFilePath(self.hdf_path.clone()));
        }
    }

    /// Construct the run string using the AT-TPC DAQ format
    fn get_run_str(&self, run_number: i32) -> String {
        return format!("run_{:0>4}", run_number);
    }
}
