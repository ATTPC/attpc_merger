use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::error::ConfigError;

/// Structure representing the application configuration. Contains pathing and run information
/// Configs are seralizable and deserializable to YAML using serde and serde_yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub graw_path: PathBuf,
    pub online: bool,
    pub experiment: String,
    pub merge_atttpc: bool,
    pub merge_silicon: bool,
    pub evt_path: Option<PathBuf>,
    pub hdf_path: PathBuf,
    pub copy_path: Option<PathBuf>,
    pub delete_copied: bool,
    pub channel_map_path: Option<PathBuf>,
    pub first_run_number: i32,
    pub last_run_number: i32,
    pub n_threads: i32,
}

impl Default for Config {
    /// Generate a new Config object. All fields will be empty/invalid
    fn default() -> Self {
        Self {
            graw_path: PathBuf::from("None"),
            online: false,
            experiment: String::from(""),
            merge_atttpc: true,
            merge_silicon: true,
            evt_path: None,
            hdf_path: PathBuf::from("None"),
            copy_path: None,
            delete_copied: true,
            channel_map_path: None,
            first_run_number: 0,
            last_run_number: 0,
            n_threads: 1,
        }
    }
}

impl Config {
    /// Read the configuration in a YAML file
    /// Returns a Config if successful
    pub fn read_config_file(config_path: &Path) -> Result<Self, ConfigError> {
        if !config_path.exists() {
            return Err(ConfigError::BadFilePath(config_path.to_path_buf()));
        }

        let yaml_str = std::fs::read_to_string(config_path)?;

        Ok(serde_yaml::from_str::<Self>(&yaml_str)?)
    }

    /// Check if a specific run exists by evaluating the existance of GET DAQ data
    /// FRIBDAQ data is optional
    pub fn does_run_exist(&self, run_number: i32) -> bool {
        let run_dir: PathBuf = self.graw_path.join(self.get_run_str(run_number));
        if self.online {
            // Don't check run_dir if online
            return true;
        }
        run_dir.exists()
    }

    /// Get the Path to a run file
    pub fn get_run_directory(&self, run_number: i32, cobo: &u8) -> Result<PathBuf, ConfigError> {
        let mut run_dir: PathBuf = self.graw_path.join(self.get_run_str(run_number));
        run_dir = run_dir.join(format!("mm{cobo}"));
        if run_dir.exists() {
            Ok(run_dir)
        } else {
            Err(ConfigError::BadFilePath(run_dir))
        }
    }

    /// Get the path to the online data, assuming the standard AT-TPC Server configuration
    pub fn get_online_directory(&self, run_number: i32, cobo: &u8) -> Result<PathBuf, ConfigError> {
        let mut online_dir: PathBuf = PathBuf::new().join(format!(
            "/Network/Servers/mm{cobo}.local/Users/attpc/Data/mm{cobo}"
        ));
        online_dir = online_dir.join(&self.experiment);
        online_dir = online_dir.join(self.get_run_str(run_number));
        if online_dir.exists() {
            Ok(online_dir)
        } else {
            Err(ConfigError::BadFilePath(online_dir))
        }
    }

    /// Get the path to the FRIBDAQ directory, assuming the standard AT-TPC configuration
    pub fn get_evt_directory(&self, run_number: i32) -> Result<Option<PathBuf>, ConfigError> {
        if self.evt_path.is_none() {
            return Ok(None);
        }
        let run_dir: PathBuf = self
            .evt_path
            .as_ref()
            .unwrap()
            .join(format!("run{run_number}"));
        if run_dir.exists() {
            Ok(Some(run_dir))
        } else {
            Err(ConfigError::BadFilePath(run_dir))
        }
    }

    /// Get the path to the output hdf5 file
    pub fn get_hdf_file_name(&self, run_number: i32) -> Result<PathBuf, ConfigError> {
        let hdf_file_path: PathBuf = self
            .hdf_path
            .join(format!("{}.h5", self.get_run_str(run_number)));
        if self.hdf_path.exists() {
            Ok(hdf_file_path)
        } else {
            Err(ConfigError::BadFilePath(self.hdf_path.clone()))
        }
    }

    /// Get the path to the copy directory, if it exists
    pub fn get_copy_directory(&self, run_number: i32) -> Result<Option<PathBuf>, ConfigError> {
        if self.copy_path.is_none() {
            return Ok(None);
        }
        let run_dir: PathBuf = self
            .copy_path
            .as_ref()
            .unwrap()
            .join(format!("run_{run_number}"));
        Ok(Some(run_dir))
    }

    /// Construct the run string using the AT-TPC DAQ format
    fn get_run_str(&self, run_number: i32) -> String {
        format!("run_{run_number:0>4}")
    }

    pub fn is_n_threads_valid(&self) -> bool {
        self.n_threads >= 1
    }

    pub fn has_evt_path(&self) -> bool {
        self.evt_path.is_some()
    }

    pub fn need_copy_files(&self) -> bool {
        self.copy_path.is_some()
    }

    pub fn delete_copied_files(&self) -> bool {
        self.need_copy_files() && self.delete_copied
    }
}
