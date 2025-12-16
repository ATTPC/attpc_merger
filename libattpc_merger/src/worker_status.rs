#[derive(Debug, Clone, Default)]
pub enum BarColor {
    #[default]
    CYAN,
    MAGENTA,
    RED,
    GREEN,
}

#[derive(Debug, Clone, Default)]
pub struct WorkerStatus {
    pub progress: f32,
    pub run_number: i32,
    pub worker_id: usize,
    pub color: BarColor,
}

impl WorkerStatus {
    pub fn new(progress: f32, run_number: i32, worker_id: usize, color: BarColor) -> Self {
        Self {
            progress,
            run_number,
            worker_id,
            color,
        }
    }
}
