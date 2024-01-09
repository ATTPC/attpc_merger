use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use eframe::egui::{Color32, RichText};

use crate::merger::config::Config;
use crate::merger::error::ProcessorError;

/// The UI app which inherits the eframe::App trait.
///
/// The parent for all processing.
#[derive(Debug)]
pub struct MergerApp {
    progress: Arc<Mutex<f32>>, //progress bar updating
    config: Config,
    worker: Option<JoinHandle<Result<(), ProcessorError>>>, //processing thread
}

impl MergerApp {
    /// Create the application
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        MergerApp {
            progress: Arc::new(Mutex::new(0.0)),
            config: Config::default(),
            worker: None,
        }
    }

    /// Start a processor
    fn start_worker(&mut self) {
        if self.worker.is_none() {
            let prog = self.progress.clone();
            let conf = self.config.clone();
            if let Ok(mut counter) = self.progress.lock() {
                *counter = 0.0;
            } else {
                println!("Sync error! Progress could not be reset!");
            }

            self.worker = Some(std::thread::spawn(|| {
                crate::merger::process::process(conf, prog)
            }))
        }
    }

    /// Stop the processor
    fn stop_worker(&mut self) {
        if let Some(handle) = self.worker.take() {
            match handle.join() {
                Ok(result) => match result {
                    Ok(_) => log::info!("Processor complete."),
                    Err(e) => log::error!("Processor error: {}", e),
                },
                Err(_) => {
                    log::error!("An error occurred joining the processor thread!");
                }
            }
        }
    }

    /// Write the current Config to a file
    fn write_config(&self, path: &Path) {
        if let Ok(mut conf_file) = File::create(path) {
            match serde_yaml::to_string(&self.config) {
                Ok(yaml_str) => match conf_file.write(yaml_str.as_bytes()) {
                    Ok(_) => (),
                    Err(x) => log::error!("Error writing config to file{}: {}", path.display(), x),
                },
                Err(x) => log::error!(
                    "Unable to write configuration to file, serializer error: {}",
                    x
                ),
            };
        } else {
            log::error!("Could not open file {} for config write", path.display());
        }
    }

    /// Read the Config from a file
    fn read_config(&mut self, path: &Path) {
        match Config::read_config_file(path) {
            Ok(conf) => self.config = conf,
            Err(e) => log::error!("{}", e),
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
                ui.add(
                    eframe::egui::widgets::DragValue::new(&mut self.config.first_run_number)
                        .speed(1),
                );
                ui.end_row();

                ui.label("Last Run Number");
                ui.add(
                    eframe::egui::widgets::DragValue::new(&mut self.config.last_run_number)
                        .speed(1),
                );
                ui.end_row()
            });

            //Controls
            // You can only click run if there isn't already someone working
            if ui
                .add_enabled(self.worker.is_none(), eframe::egui::Button::new("Run"))
                .clicked()
            {
                log::info!("Starting processor...");
                self.start_worker();
            } else {
                match self.worker.as_ref() {
                    Some(worker) => {
                        if worker.is_finished() {
                            self.stop_worker()
                        }
                    }
                    None => (),
                }
            }

            //Progress Bar
            ui.separator();
            ui.label(
                RichText::new("Progress")
                    .color(Color32::LIGHT_BLUE)
                    .size(18.0),
            );
            ui.add(
                eframe::egui::widgets::ProgressBar::new(match self.progress.lock() {
                    Ok(x) => *x,
                    Err(_) => 0.0,
                })
                .show_percentage(),
            );

            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        });
    }
}
