use std::path::{Path, PathBuf};

use super::config::Config;
use super::constants::{COBO_OF_SILICON, NUMBER_OF_COBOS};
use super::error::FileCopierError;

/// FileCopier copy graw and evt files from network to local storage.
pub struct FileCopier {
    file_stack: Vec<(PathBuf, PathBuf, u64)>,
    total_data_size_bytes: u64,
}

impl FileCopier {
    /// Create a new FileCopier.
    ///
    /// Requires config file and run number. It will break if the copy directory is None.
    pub fn new(config: &Config, run_number: i32) -> Result<Self, FileCopierError> {
        if !config.need_copy_files() {
            return Ok(Self {
                file_stack: vec![],
                total_data_size_bytes: 0,
            });
        }
        let mut stack: Vec<(PathBuf, PathBuf, u64)> = Vec::new();
        let mut total_size: u64 = 0;
        let copy_dir = config.get_copy_directory(run_number)?.unwrap();
        if let Ok(Some(evt_file)) = config.get_evt_directory(run_number) {
            if let Ok(file_list) = Self::get_file_stack(&evt_file, "run-", ".evt") {
                for (path, bytes) in file_list {
                    let src = path.clone();
                    let dst = copy_dir.join("evt").join(path.file_name().unwrap());
                    stack.push((src, dst, bytes));
                    total_size += bytes;
                }
            }
        }

        for cobo in 0..NUMBER_OF_COBOS {
            if !config.merge_atttpc && cobo < COBO_OF_SILICON {
                continue;
            }
            if !config.merge_silicon && cobo >= COBO_OF_SILICON {
                continue;
            }
            let graw_dir = if config.online {
                config.get_online_directory(run_number, &cobo)?
            } else {
                config.get_run_directory(run_number, &cobo)?
            };
            if let Ok(file_list) =
                Self::get_file_stack(&graw_dir, &format!("CoBo{}_AsAd", cobo), ".graw")
            {
                for (path, bytes) in file_list {
                    let src = path.clone();
                    let dst = copy_dir
                        .join(format!("mm{cobo}"))
                        .join(path.file_name().unwrap());
                    stack.push((src, dst, bytes));
                    total_size += bytes;
                }
            }
        }
        Ok(Self {
            file_stack: stack,
            total_data_size_bytes: total_size,
        })
    }

    /// Get file stack function adapted for both graw and evt file.
    fn get_file_stack(
        parent_path: &Path,
        start_pattern: &str,
        end_pattern: &str,
    ) -> Result<Vec<(PathBuf, u64)>, FileCopierError> {
        let mut file_list: Vec<(PathBuf, u64)> = Vec::new();
        for item in parent_path.read_dir()? {
            let item_path = item?.path();
            let item_path_str = item_path.to_str().unwrap();
            if item_path_str.contains(start_pattern) && item_path_str.contains(end_pattern) {
                let bytes = item_path.metadata().unwrap().len();
                file_list.push((item_path, bytes));
            }
        }
        Ok(file_list)
    }

    /// Get total copy size of files.
    pub fn get_total_data_size(&self) -> u64 {
        self.total_data_size_bytes
    }

    /// Get source, destination, size for copying process.
    ///
    /// This function retuns list of source path, destination path and size in bytes.
    /// Usually used in loop.
    pub fn copy_meta(&self) -> &Vec<(PathBuf, PathBuf, u64)> {
        &self.file_stack
    }
}
