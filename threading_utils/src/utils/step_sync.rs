use std::{sync::{atomic::{AtomicUsize, Ordering}, Arc, Condvar, Mutex}, thread, time::Duration};

#[derive(Clone)]
pub struct StepSync {
    cond:Arc<(Mutex<bool>, Condvar)>,
    number_synced:Arc<AtomicUsize>,
}

impl StepSync {
    pub fn new() -> Self {
        Self { cond:Arc::new((Mutex::new(false),Condvar::new())), number_synced:Arc::new(AtomicUsize::new(0)) }
    }
    pub fn start_action(&self, number_to_sync:usize) {
        *self.cond.0.lock().unwrap() = false;
    }

    pub fn wait_here(&self, number_to_sync:usize) {
        if number_to_sync > 1 {
            self.number_synced.fetch_add(1, Ordering::Relaxed);
            if self.number_synced.load(Ordering::Relaxed) < number_to_sync {
                let mut started = self.cond.0.lock().unwrap();
                while !*started {
                    started = self.cond.1.wait(started).unwrap();
                }
            }
            else {
                *self.cond.0.lock().unwrap() = true;
                self.cond.1.notify_all();
                self.number_synced.store(0, Ordering::Relaxed);
                
            }
        }
    }
}