use std::sync::{Arc, RwLock};

pub type ARW<T> = Arc<RwLock<T>>;

pub mod parallel_counter;
pub mod crz_op;
pub mod bitfield;
pub mod array_vec;
pub mod mpsc_vec;
pub mod dynamic_mpmc_consumer;
pub mod late_alloc_mpmc_vec;