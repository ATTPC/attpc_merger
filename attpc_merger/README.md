# attpc_merger_cli

Part of the attpc_merger crate family.

This is the application to merge AT-TPC data with a GUI using [egui](https://github.com/emilk/egui).

## Install

Use `cargo install attpc_merger`

## Use

To launch the application simply invoke it after it is installed

```bash
attpc_merger
```

Fill out the configuration fields and click the run button to merge data.

## Configuration

The following configuration controls are available in the GUI:

- GRAW Files from Online Source Checkbox: If checked, will try to access GRAW files from the AT-TPC DAQ itself. Should typically be unchecked. Only for use when running an experiment.
  - Experiment: Only available when online checkbox is checked. Name of the experiment, matching the AT-TPC DAQ experiment name.
- GRAW directory: Specifies the full-path to a directory which contains the AT-TPC GETDAQ .graw structure (i.e. contains subdirectories of the run_# format). If online is checked, this field is not available.
- EVT directory: Specifies the full-path to a directory which contains the FRIBDAQ EVT structure (i.e. contains subdirectories of the run# format)
- HDF5 directory: Specifies the full-path to a directory to which merged HDF5 (.h5) files will be written
- Pad map: Specifies the full path to a CSV file which contains the mapping information for AT-TPC pads and electronics
- First Run Number: The starting run number (inclusive)
- Last Run Number: The ending run number (inclusive)

Configurations can be saved using File->Save and loaded using File->Open
