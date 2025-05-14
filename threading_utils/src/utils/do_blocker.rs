use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Condvar, Mutex};

pub struct DoBlocker {
    cond:(Mutex<bool>, Condvar),
    on_it:AtomicBool,
}

impl DoBlocker {
    pub fn new() -> Self {
        Self { cond:(Mutex::new(false),Condvar::new()), on_it:AtomicBool::new(false) }
    }
    pub fn wait_if_on_it(&self, val:bool) -> bool {
        if self.should_wait(val) {
            *self.cond.0.lock().unwrap() = false;
            true
        }
        else {
            self.wait();
            false
        }
    }
    pub fn stop_waiting(&self) {
        *self.cond.0.lock().unwrap() = true;
        self.on_it.store(false, Ordering::Relaxed);
        self.cond.1.notify_all();
    }
    pub fn should_wait(&self, val:bool) -> bool {
        self.on_it.fetch_or(val, Ordering::Relaxed) || !val
    }
    pub fn should_theoretically_wait(&self, val:bool) -> bool {
        self.on_it.load(Ordering::Relaxed) || !val
    }
    pub fn anyone_on_it(&self) -> bool {
        self.on_it.load(Ordering::Relaxed)
    }
    pub fn start_wait(&self) {
        *self.cond.0.lock().unwrap() = false;
        //self.on_it.store(true, Ordering::Relaxed);
    }
    pub fn wait(&self) {
        let mut started = self.cond.0.lock().unwrap();
        while !*started || self.on_it.load(Ordering::Relaxed) {
            started = self.cond.1.wait(started).unwrap();
        }
    }
}