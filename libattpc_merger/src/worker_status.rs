#[derive(Debug, Clone, Default)]
pub struct WorkerStatus {
    pub progress: f32,
    pub run_number: i32,
    pub worker_id: usize,
}

impl WorkerStatus {
    pub fn new(progress: f32, run_number: i32, worker_id: usize) -> Self {
        Self {
            progress,
            run_number,
            worker_id,
        }
    }
}
