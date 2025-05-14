use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, thread::{self, sleep}, time::Duration};

use rand::{thread_rng, Rng};

use crate::utils::step_sync::StepSync;


const N_THREADS:usize = 25600;
const OPS_RANGE:(usize,usize) = (10000000, 100000000);
const N_SYNCS:usize = 200000;

#[test]
fn step_sync_test() {
    let sync = StepSync::new();
    let mut total = Arc::new(AtomicUsize::new(0));
    let mut ops = Arc::new(AtomicUsize::new(0));
    for i in 0..N_THREADS {
        let sync_clone = sync.clone();
        let total_clone = total.clone();
        let ops_clone = ops.clone();
        thread::spawn(move || {
            let mut rng = thread_rng();

            for j in 0..N_SYNCS {
                sync_clone.start_action(N_THREADS);
                ops_clone.store(rng.gen_range(OPS_RANGE.0..OPS_RANGE.1), Ordering::Relaxed);
                sync_clone.wait_here(N_THREADS);

                sync_clone.start_action(N_THREADS);
                let n_ops = ops_clone.load(Ordering::Relaxed) * N_THREADS;
                let prediction = total_clone.load(Ordering::Relaxed) + n_ops;
                sync_clone.wait_here(N_THREADS);

                sync_clone.start_action(N_THREADS);
                for op in 0.. {
                    total_clone.fetch_add(1, Ordering::Relaxed);
                }
                sync_clone.wait_here(N_THREADS);
                assert!(total_clone.load(Ordering::Relaxed) == prediction);
            }
        });
    }

}