//! # attpc_merger
//!
//! ![CI](https://github.com/ATTPC/attpc_engine/actions/workflows/ci.yml/badge.svg)
//!
//! attpc_merger is the AT-TPC event builder, written in Rust. It takes data produced by
//! the AT-TPC data acquisition in the form of .graw files from the GET system and .evt
//! files from the FRIBDAQ system, and combines them into a single unified event structure
//! in the HDF5 format.
//!
//! ## Installation
//!
//! In the future we may depoly to crates.io, but currently the only method of install is
//! from source, which is laid out below.
//!
//! ### Rust
//!
//! If you have not used Rust before, you will most likely need to install the Rust tool
//! chain. See the [Rust docs](https://www.rust-lang.org/tools/install) for installation
//! instructions.
//!
//! ### Platform Support
//!
//! attpc_merger aims to support Linux, MacOS, and Windows. Currently, attpc_merger has
//! been tested and built successfully on Ubuntu 22.04 and MacOS 13 (Ventura).
//!
//! ### Downloading
//!
//! To download attpc_merger clone the git repository using
//! `git clone https://github.com/attpc/attpc_merger.git`
//!
//! ### HDF5
//!
//! Before building and running attpc_merger, HDF5 must be installed. Typically this will
//! be installed using a package manager (homebrew, apt, etc), and the Rust libraries will
//! auto detect the location of the HDF install. However, this is not always possible.
//! Sometimes a newer version will need to be installed to a custom location. If this is
//! the case, write the following snippet into the file `.cargo/config.toml` in the
//! attpc_merger repository:
//!
//! ```toml
//! [env]
//! HDF5_DIR="/path/to/my/hdf5/install/"
//!
//! [build]
//! rustflags="-C link-args=-Wl,-rpath,/path/to/my/hdf5/install/lib"
//! ```
//!
//! Replace `/path/to/my/hdf5/install/` with the path to your HDF5 installation. The extra
//! build command assumes that the hdf5 files are not installed to the normal library
//! search path of your operating sytsem. Note that you will need to create the `.cargo`
//! directory and the `config.toml` file.
//!
//! ### Building & Install
//!
//! To build and install the GUI merger use `cargo install --path ./attpc_merger` from the
//! top level attpc_merger repository.
//!
//! To build and install the CLI merger use `cargo install --path ./attpc_merger_cli` from
//! the top level attpc_merger repository.
//!
//! These binaries will be installed to your cargo install location (typically something
//! like `~/.cargo/bin/`). They can be uninstalled by running
//! `cargo uninstall attpc_merger/_cli`. Once they are installed, they will be in your
//! path, so you can simply invoke them from the command line. To use the CLI see the
//! `attpc_merger_cli` README.
//!
//! ## Documentation
//!
//! Documentation is deployed through GitHub Pages and can be found
//! [here](https://attpc.github.io/attpc_merger). Documentation is provided for the
//! `libattpc_merger` library; the source code of the `attpc_merger\_cli` applications
//! should be examined for details on the UI design and examples of using the merger library.
//!
//! ## Configuration
//!
//! The following configuration controls are available in the GUI:
//!
//! - GRAW Files from Online Source Checkbox: If checked, will try to access GRAW files
//! from the AT-TPC DAQ itself. Should typically be unchecked. Only for use when running an
//! experiment.
//!   - Experiment: Only available when online checkbox is checked. Name of the experiment, matching the AT-TPC DAQ experiment name.
//! - GRAW directory: Specifies the full-path to a directory which contains the AT-TPC
//! GETDAQ .graw structure (i.e. contains subdirectories of the run_# format). If online
//! is checked, this field is not available.
//! - EVT directory: Specifies the full-path to a directory which contains the FRIBDAQ EVT
//! structure (i.e. contains subdirectories of the run# format)
//! - HDF5 directory: Specifies the full-path to a directory to which merged HDF5 (.h5)
//! files will be written
//! - Channel map (Optional): Specifies the full path to a CSV file which contains the
//! mapping information for AT-TPC GET detectors and electronics. If set to default
//! (clicking the Default button), it will use a channel map that has been bundled with the
//! code base.
//! - First Run Number: The starting run number (inclusive)
//! - Last Run Number: The ending run number (inclusive)
//! - Number of Workers: The number of parallel worker threads to divide the runs amongst.
//! Each worker will get a subset of the run range. If you don't have enough runs to give
//! all workers something to do, only the threads that would do work are created (i.e.
//! n_workers = 3, n_runs = 2, only 2 workers are created). Must be at least 1.
//!
//! Configurations can be saved using File->Save and loaded using File->Open
//!
//! A configuration file saved using the UI is compatible with the CLI and vice-versa. The
//! YAML format of a configuration file is as follows:
//!
//! ```yml
//! graw_path: None
//! evt_path: None
//! hdf_path: None
//! channel_map_path: null
//! first_run_number: 0
//! last_run_number: 0
//! online: false
//! experiment: ''
//! n_threads: 1
//! ```
//!
//! Note that if the `channel_map_path` field is set to `null`, the bundled default map
//! will be used.
//!
//! ### Channel Map Format
//!
//! The channel map is a CSV file with *no* whitespaces. The columns are as follows:
//!
//! ```csv
//! cobo,asad,aget,aget channel,det keyword,det channel
//! ```
//!
//! The CoBo, AsAd, AGET, and channel columns denote the GET hardware for that specific channel.
//! The `det keyword` and `det channel` columns are a unique keyword and channel eferring
//! to which detector channel the electronics are connected to. Currently supported keywords
//! are:
//!
//! - `pad`
//! - `si_upstream_front`
//! - `si_upstream_back`
//! - `si_downstream_front`
//! - `si_downstream_back`
//!
//! ## Output
//!
//! attpc_merger will output two files: the final resulting HDF5 data file, and a log file.
//! Log files contain valuable information about the status of the application while
//! building the merged data. If an error occurs, typically a warning will be printed to
//! the terminal indicating that the user should check the log file. The log file will
//! contain the detailed status of the run and indicate the issue that occurred. Log files
//! are also useful because they can be easily shared when errors occur. It is not advised
//! to delete the log files.
//!
//! ### HDF5 Data Format
//!
//! The data format used in the HDF5 data is as follows:
//!
//! ```text
//! run_0001.h5
//! events - min_event, max_event, min_get_ts, max_get_ts, frib_run, frib_start, frib_stop, frib_time, version
//! |---- event_#
//! |    |---- get - id, timestamp, timestamp_other
//! |    |    |---- pads(dset)
//! |    |    |---- si_upstream_front(dset)
//! |    |    |---- si_upstream_back(dset)
//! |    |    |---- si_downstream_front(dset)
//! |    |    |---- si_downstream_back(dset)
//! |    |---- frib_physics - id, timestamp
//! |    |    |---- 907(dset)
//! |    |    |---- 1903(dset)
//! scalers - min_event, max_event
//! |---- event_#(dset) - start_offset, stop_offset, timestamp, incremental
//! ```
pub mod asad_stack;
pub mod channel_map;
pub mod config;
pub mod constants;
pub mod error;
pub mod event;
pub mod event_builder;
pub mod evt_file;
pub mod evt_stack;
pub mod graw_file;
pub mod graw_frame;
pub mod hardware_id;
pub mod hdf_writer;
pub mod merger;
pub mod process;
pub mod ring_item;
pub mod worker_status;
