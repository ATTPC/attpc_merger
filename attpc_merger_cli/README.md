# attpc_merger_cli

Part of the attpc_merger crate family.

This is the CLI application to merge AT-TPC data.

## Install

Use `cargo install attpc_merger_cli`

## Use

To merge data use the following command

```bash
attpc_merger_cli -p/--path <your_configuration.yaml> 
```

To generate a configuration template file use

```bash
attpc_merger_cli -p/--path <your_configuration.yaml> new
```

## Configuration

The following fields must be specified in the configuration file:

- graw_path: Specifies the full-path to a directory which contains the AT-TPC GETDAQ GRAW structure (i.e. contains subdirectories of the run_# format)
- evt_path: Specifies the full-path to a directory which contains the FRIBDAQ EVT structure (i.e. contains subdirectories of the run# format)
- hdf_path: Specifies the full-path to a directory to which merged HDF5 (.h5) files will be written
- pad_map_path: Specifies the full path to a CSV file which contains the mapping information for AT-TPC pads and electronics
- first_run_number: The starting run number (inclusive)
- last_run_number: The ending run number (inclusive)
- online: Boolean flag indicating if online data sources should be used (overrides some of the path imformation); generally should be false
- experiment: Experiment name as a string. Only used when online is true. Should match the experiment name used by the AT-TPC DAQ.
