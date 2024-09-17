use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use eframe::egui::{Color32, DragValue, ProgressBar, RichText};

use libattpc_merger::config::Config;
use libattpc_merger::error::ProcessorError;
use libattpc_merger::process::{create_subsets, process_subset};

/// The UI app which inherits the eframe::App trait.
///
/// The parent for all processing.
#[derive(Debug)]
pub struct MergerApp {
    progresses: Vec<Arc<Mutex<f32>>>, //progress bar updating
    config: Config,
    workers: Vec<JoinHandle<Result<(), ProcessorError>>>, //processing thread
    current_runs: Vec<Arc<Mutex<i32>>>,
}

impl MergerApp {
    /// Create the application
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        MergerApp {
            progresses: vec![],
            config: Config::default(),
            workers: vec![],
            current_runs: vec![],
        }
    }

    /// Start some workers
    fn start_workers(&mut self) {
        if self.workers.is_empty() {
            self.progresses.clear();
            self.current_runs.clear();
            let subsets = create_subsets(&self.config);
            for subset in subsets {
                // Dont make empty workers
                if subset.is_empty() {
                    continue;
                }
                // Create all of this worker's info
                let stat = Arc::new(Mutex::new(0.0));
                let run = Arc::new(Mutex::new(0));
                // Spawn it
                let conf = self.config.clone();
                self.progresses.push(stat.clone());
                self.current_runs.push(run.clone());
                self.workers.push(std::thread::spawn(|| {
                    process_subset(conf, stat, run, subset)
                }))
            }
        }
    }

    /// Stop the processor
    fn stop_workers(&mut self) {
        let n_workers = self.workers.len();
        for _ in 0..n_workers {
            if let Some(worker) = self.workers.pop() {
                match worker.join() {
                    Ok(res) => match res {
                        Ok(_) => spdlog::info!("Worker complete"),
                        Err(e) => spdlog::error!("Processor error: {e}"),
                    },
                    Err(_) => spdlog::error!("An error occured joining one of the workers!"),
                }
            }
        }
    }

    fn are_any_workers_alive(&self) -> bool {
        for worker in self.workers.iter() {
            if !worker.is_finished() {
                return true;
            }
        }
        return false;
    }

    /// Write the current Config to a file
    fn write_config(&self, path: &Path) {
        if let Ok(mut conf_file) = File::create(path) {
            match serde_yaml::to_string(&self.config) {
                Ok(yaml_str) => match conf_file.write(yaml_str.as_bytes()) {
                    Ok(_) => (),
                    Err(x) => {
                        spdlog::error!("Error writing config to file{}: {}", path.display(), x)
                    }
                },
                Err(x) => spdlog::error!(
                    "Unable to write configuration to file, serializer error: {}",
                    x
                ),
            };
        } else {
            spdlog::error!("Could not open file {} for config write", path.display());
        }
    }

    /// Read the Config from a file
    fn read_config(&mut self, path: &Path) {
        match Config::read_config_file(path) {
            Ok(conf) => self.config = conf,
            Err(e) => spdlog::error!("{}", e),
        }
    }
}

impl eframe::App for MergerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            //Menus
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml"])
                        .show_open_single_file()
                    {
                        self.read_config(&path);
                    }
                }
                if ui.button("Save...").clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml"])
                        .show_save_single_file()
                    {
                        self.write_config(&path);
                    }
                }
            });

            //Config
            ui.separator();
            ui.label(
                RichText::new("Configuration")
                    .color(Color32::LIGHT_BLUE)
                    .size(18.0),
            );
            eframe::egui::Grid::new("ConfigGrid").show(ui, |ui| {
                //GRAW directory
                ui.checkbox(&mut self.config.online, "GRAW files from online source");
                ui.end_row();
                //Online data requires a further path extension based on the experiment
                if self.config.online {
                    ui.label("Experiment:");
                    ui.text_edit_singleline(&mut self.config.experiment);
                    ui.end_row();
                } else {
                    ui.label(format!(
                        "GRAW directory: {}",
                        self.config.graw_path.display()
                    ));
                    if ui.button("Open...").clicked() {
                        if let Ok(Some(path)) = native_dialog::FileDialog::new()
                            .set_location(
                                &std::env::current_dir()
                                    .expect("Couldn't access runtime directory"),
                            )
                            .show_open_single_dir()
                        {
                            self.config.graw_path = path;
                        }
                    }
                    ui.end_row();
                }

                //EVT directory
                ui.label(format!("EVT directory: {}", self.config.evt_path.display()));
                if ui.button("Open...").clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access evt directory"),
                        )
                        .show_open_single_dir()
                    {
                        self.config.evt_path = path;
                    }
                }
                ui.end_row();

                //HDF directory
                ui.label(format!(
                    "HDF5 directory: {}",
                    self.config.hdf_path.display()
                ));
                if ui.button("Open...").clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .show_open_single_dir()
                    {
                        self.config.hdf_path = path;
                    }
                }
                ui.end_row();

                //Pad map
                ui.label(format!("Pad map: {}", self.config.pad_map_path.display()));
                if ui.button("Open...").clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("CSV file", &["csv", "CSV", "txt"])
                        .show_open_single_file()
                    {
                        self.config.pad_map_path = path;
                    }
                }
                ui.end_row();

                ui.label("First Run Number");
                ui.add(DragValue::new(&mut self.config.first_run_number).speed(1));
                ui.end_row();

                ui.label("Last Run Number");
                ui.add(DragValue::new(&mut self.config.last_run_number).speed(1));
                ui.end_row();

                ui.label("Number of Workers");
                ui.add(DragValue::new(&mut self.config.n_threads).speed(1));
                ui.end_row();
            });

            //Controls
            // You can only click run if there isn't already someone working
            if ui
                .add_enabled(self.workers.is_empty(), eframe::egui::Button::new("Run"))
                .clicked()
            {
                spdlog::info!("Starting processor...");
                self.start_workers();
            } else if !self.are_any_workers_alive() {
                self.stop_workers();
            }

            //Progress Bars
            ui.separator();
            ui.label(
                RichText::new("Progress Per Worker")
                    .color(Color32::LIGHT_BLUE)
                    .size(18.0),
            );
            for (idx, progress) in self.progresses.iter().enumerate() {
                let crun = match self.current_runs[idx].lock() {
                    Ok(r) => *r,
                    Err(_) => 0,
                };
                let prog = match progress.lock() {
                    Ok(p) => *p,
                    Err(_) => 0.0,
                };
                ui.add(ProgressBar::new(prog).text(format!(
                    "Worker {} : Run {} - {}%",
                    idx,
                    crun,
                    (prog * 100.0) as i32
                )));
            }

            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        });
    }
}
