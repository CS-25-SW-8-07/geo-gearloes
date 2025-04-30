use std::sync::Arc;

use eframe::egui::mutex::Mutex;
use geo::{LineString, Point};
use rayon::prelude::*;

pub type AWorkers = Arc<Workers>;

pub struct Workers {
    count: usize,
    threads: usize,
    workers: Vec<Worker>,
}

impl Workers {
    pub fn new(count: usize, threads: usize, spawn: impl Fn(usize, usize) -> Worker) -> Arc<Self> {
        let spawn = &spawn;
        let workers = (0..threads)
            .flat_map(|t| (0..count).map(move |c| spawn(c, t)))
            .collect::<Vec<_>>();

        Arc::new(Self {
            count,
            threads,
            workers,
        })
    }

    pub fn step_all(&self) {
        self.workers
            .par_windows(self.count)
            .for_each(|window| window.iter().for_each(|w| w.step()));
    }
}

#[derive(Debug, Default, Clone)]
pub struct WorkerState {
    pos: Point,
    intent: Option<LineString>,
}

pub struct Worker {
    state: Arc<Mutex<WorkerState>>,
}

impl Worker {
    pub fn new() -> Self {
        Worker {
            state: Default::default(),
        }
    }

    pub fn step(&self) {
        let mut state = self.state.lock();
        let state_ref = &mut state;
        // TODO: Step worker
    }
}

unsafe impl Send for Worker {}
unsafe impl Sync for Worker {}
