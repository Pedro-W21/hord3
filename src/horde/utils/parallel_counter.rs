use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

#[derive(Clone)]
pub struct ParallelCounter {
    personal_counter:usize,
    common_counter:Arc<AtomicUsize>,
    length:usize,
    atomic_length:Arc<AtomicUsize>,
    current_objective:usize,
    take_size:usize,
    atomic_buffer_size:Arc<AtomicUsize>
}

impl ParallelCounter {
    pub fn reset(&self) {
        self.common_counter.store(0, Ordering::Relaxed);
    }
    pub fn initialise(&mut self) {
        self.length = self.atomic_length.load(Ordering::Relaxed);
        self.take_size = self.atomic_buffer_size.load(Ordering::Relaxed);
        self.current_objective = self.common_counter.fetch_add(self.take_size, Ordering::SeqCst) + self.take_size;
        self.personal_counter = self.current_objective - self.take_size;
    }
    pub fn new(length:usize, buffer_size:usize) -> Self {
        Self { personal_counter: 0, common_counter: Arc::new(AtomicUsize::new(0)), length, atomic_length: Arc::new(AtomicUsize::new(length)), current_objective: 0, take_size: buffer_size, atomic_buffer_size: Arc::new(AtomicUsize::new(buffer_size)) }
    }
    pub fn update_len(&self, len:usize) {
        self.atomic_length.store(len, Ordering::Relaxed);
    }
    pub fn update_buffer_size(&self, size:usize) {
        self.atomic_buffer_size.store(size, Ordering::Relaxed);
    }
    pub fn print_common_counter(&self) {
        println!("{}", self.common_counter.load(Ordering::Relaxed));
    }
}

impl Iterator for ParallelCounter {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let return_value = self.personal_counter;
        self.personal_counter += 1;
        //println!("{} {}", self.current_objective, self.personal_counter);
        
        if self.personal_counter >= self.current_objective {
            //println!("added stuff");
            self.current_objective = self.common_counter.fetch_add(self.take_size, Ordering::SeqCst) + self.take_size;
            self.personal_counter = self.current_objective - self.take_size;
            if self.current_objective > self.length {
                self.current_objective = self.length;
            }
        }
        if self.length > return_value {
            //if self.length == 1 {
            //    println!("{} {} {}", self.current_objective, self.personal_counter, return_value);
            //}
            Some(return_value)
        }
        else {
            None
        }
        
    }
}