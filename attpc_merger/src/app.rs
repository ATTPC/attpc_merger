use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;

use eframe::egui::{Color32, DragValue, ProgressBar, RichText};
use rfd::FileDialog;

use libattpc_merger::config::Config;
use libattpc_merger::error::ProcessorError;
use libattpc_merger::process::{create_subsets, process_subset};
use libattpc_merger::worker_status::{BarColor, WorkerStatus};

fn render_error_dialog(show: &mut bool, ctx: &eframe::egui::Context) {
    eframe::egui::Window::new("Error")
        .open(show)
        .show(ctx, |ui| {
            ui.label(
                "There was an error! Check the log file attpc_merger.log for more information.",
            )
        });
}

/// The UI app which inherits the eframe::App trait.
///
/// The parent for all processing.
#[derive(Debug)]
pub struct MergerApp {
    config: Config,
    workers: Vec<JoinHandle<Result<(), ProcessorError>>>, //processing thread
    worker_statuses: Vec<WorkerStatus>,
    show_error_window: bool,
    worker_rx: mpsc::Receiver<WorkerStatus>,
    worker_tx: mpsc::Sender<WorkerStatus>,
}

impl MergerApp {
    /// Create the application
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = eframe::egui::Visuals::dark();
        visuals.override_text_color = Some(Color32::LIGHT_GRAY);
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);
        let (tx, rx) = mpsc::channel::<WorkerStatus>();
        MergerApp {
            config: Config::default(),
            workers: vec![],
            worker_statuses: vec![],
            show_error_window: false,
            worker_rx: rx,
            worker_tx: tx,
        }
    }

    /// Start some workers
    fn start_workers(&mut self) {
        // Safety first
        if self.workers.is_empty() {
            self.worker_statuses.clear();
            let subsets = create_subsets(&self.config);
            for (idx, subset) in subsets.into_iter().enumerate() {
                // Dont make empty workers
                if subset.is_empty() {
                    continue;
                }
                // Spawn it
                let conf = self.config.clone();
                let tx = self.worker_tx.clone();
                let bar_color = if self.config.need_copy_files() {
                    BarColor::GREEN
                } else {
                    BarColor::CYAN
                };
                self.worker_statuses
                    .push(WorkerStatus::new(0.0, 0, idx, bar_color));
                self.workers.push(std::thread::spawn(move || {
                    process_subset(conf, tx, idx, subset)
                }))
            }
        }
    }

    /// Stop the workers
    fn stop_workers(&mut self) {
        let n_workers = self.workers.len();
        for _ in 0..n_workers {
            if let Some(worker) = self.workers.pop() {
                match worker.join() {
                    Ok(res) => match res {
                        Ok(_) => spdlog::info!("Worker complete"),
                        Err(e) => {
                            self.show_error_window = true;
                            spdlog::error!("Processor error: {e}")
                        }
                    },
                    Err(_) => {
                        self.show_error_window = true;
                        spdlog::error!("An error occured joining one of the workers!")
                    }
                }
            }
        }
    }

    /// Check if there are any workers still doing stuff
    fn are_any_workers_alive(&self) -> bool {
        for worker in self.workers.iter() {
            if !worker.is_finished() {
                return true;
            }
        }
        false
    }

    /// Write the current Config to a file
    fn write_config(&mut self, path: &Path) {
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
            self.show_error_window = true;
            spdlog::error!("Could not open file {} for config write", path.display());
        }
    }

    fn poll_messages(&mut self) {
        // Check messages
        loop {
            match self.worker_rx.try_recv() {
                Ok(status) => {
                    let id = status.worker_id;
                    self.worker_statuses[id] = status;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    spdlog::error!("Channels became disconnected!");
                    self.show_error_window = true;
                }
            }
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
        self.poll_messages();
        render_error_dialog(&mut self.show_error_window, ctx);
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            //Menus
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml", "yml"])
                        .pick_file()
                    {
                        self.read_config(&path);
                    }
                }
                if ui.button("Save...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml", "yml"])
                        .save_file()
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
                        if let Some(path) = FileDialog::new()
                            .set_directory(
                                std::env::current_dir().expect("Couldn't access runtime directory"),
                            )
                            .pick_folder()
                        {
                            self.config.graw_path = path;
                        }
                    }
                    ui.end_row();
                }
                ui.checkbox(&mut self.config.merge_atttpc, "Merge AT-TPC data");
                ui.checkbox(&mut self.config.merge_silicon, "Merge Silicon data");
                ui.end_row();

                //EVT directory
                ui.label(format!(
                    "EVT directory: {}",
                    self.config
                        .evt_path
                        .clone()
                        .unwrap_or(PathBuf::from("None"))
                        .display()
                ));
                if ui.button("Open...").clicked() {
                    self.config.evt_path = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access evt directory"),
                        )
                        .pick_folder();
                }
                ui.end_row();

                //HDF directory
                ui.label(format!(
                    "HDF5 directory: {}",
                    self.config.hdf_path.display()
                ));
                if ui.button("Open...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .pick_folder()
                    {
                        self.config.hdf_path = path;
                    }
                }
                ui.end_row();

                // Copy file
                ui.label(format!(
                    "Copy directory: {}",
                    self.config
                        .copy_path
                        .clone()
                        .unwrap_or(PathBuf::from("None"))
                        .display()
                ));
                if ui.button("Open...").clicked() {
                    self.config.copy_path = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .pick_folder();
                }
                ui.end_row();
                ui.label("Delete copied files after merging");
                ui.checkbox(&mut self.config.delete_copied, "");
                ui.end_row();

                //Pad map
                let map_render_text: String = match &self.config.channel_map_path {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => String::from("Default"),
                };
                ui.label(format!("Pad map: {map_render_text}"));
                if ui.button("Open...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_directory(
                            std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("CSV file", &["csv", "CSV", "txt"])
                        .pick_file()
                    {
                        self.config.channel_map_path = Some(path);
                    }
                }
                if ui.button("Default").clicked() {
                    self.config.channel_map_path = None
                }
                ui.end_row();

                ui.label("First Run Number");
                ui.add(DragValue::new(&mut self.config.first_run_number).speed(1));
                ui.end_row();

                ui.label("Last Run Number");
                ui.add(DragValue::new(&mut self.config.last_run_number).speed(1));
                ui.end_row();

                ui.label("Number of Workers");
                ui.add(
                    DragValue::new(&mut self.config.n_threads)
                        .speed(1)
                        .range(std::ops::RangeInclusive::new(1, 10)),
                );
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
            for status in self.worker_statuses.iter() {
                let msg = match status.color {
                    BarColor::GREEN => "Copying",
                    BarColor::CYAN => "Merging",
                    _ => "",
                };
                let color = match status.color {
                    BarColor::GREEN => Color32::DARK_GREEN,
                    BarColor::CYAN => Color32::BLUE,
                    BarColor::MAGENTA => Color32::MAGENTA,
                    BarColor::RED => Color32::RED,
                };
                ui.add(
                    ProgressBar::new(status.progress)
                        .text(format!(
                            "Worker {} : {} run {} - {}%",
                            status.worker_id,
                            msg,
                            status.run_number,
                            (status.progress * 100.0) as i32
                        ))
                        .fill(color),
                );
            }

            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        });
    }
}
