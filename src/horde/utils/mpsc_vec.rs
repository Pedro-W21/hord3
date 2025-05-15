use std::{cell::SyncUnsafeCell, mem::MaybeUninit, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc}, thread, time::Duration};

use threading_utils::utils::do_blocker::DoBlocker;

#[derive(Clone)]
pub struct MPSCVec<T:Clone + Sync + Send> {
    inner:Arc<InnerMPSCVec<T>>
}

pub struct InnerMPSCVec<T:Clone + Sync + Send> {
    currently_consuming:AtomicBool,
    currently_pushing:AtomicUsize,
    current_len:AtomicUsize,
    current_vec_len:AtomicUsize,
    data:SyncUnsafeCell<Vec<MaybeUninit<T>>>,
    blocker:DoBlocker,
}

impl<T:Clone + Sync + Send> MPSCVec<T> {
    pub fn len(&self) -> usize {
        self.inner.current_len.load(Ordering::Acquire)
    }
    pub fn inner_vec_len(&self) -> usize {
        self.inner.current_vec_len.load(Ordering::Relaxed)
    }
    pub fn new() -> Self {
        Self { inner: Arc::new(
            InnerMPSCVec { currently_consuming: AtomicBool::new(false), current_vec_len:AtomicUsize::new(0), currently_pushing:AtomicUsize::new(0), current_len:AtomicUsize::new(0), data: SyncUnsafeCell::new(Vec::new()), blocker: DoBlocker::new() }
        ) }
    }

    /// Safety : Must NOT be used at the same time as `push`, `drop_all_elems`, `read_all_elems` or `consume_all_elems`
    pub unsafe fn get_unchecked(&self, at:usize) -> &T {
        self.inner.data.get().as_ref().unwrap()[at].assume_init_ref()
    }
    #[inline(always)]
    unsafe fn try_push_with_id(&self, val:T, id:usize) -> bool { //push again
        unsafe {
            
            let vec_len = self.inner.current_vec_len.load(Ordering::Relaxed);
            self.inner.currently_pushing.load(Ordering::Relaxed);
            if id < vec_len && !self.inner.blocker.anyone_on_it() {
                let data = self.inner.data.get().as_mut().unwrap();
                data[id] = MaybeUninit::new(val);
                self.inner.currently_pushing.fetch_sub(1, Ordering::Relaxed);
                false
            }
            else if self.inner.blocker.should_wait(id == vec_len) {
                self.inner.currently_pushing.fetch_sub(1, Ordering::Relaxed);
                self.inner.blocker.wait();
                //println!("{} {} {} {}", id, self.inner.current_vec_len.load(Ordering::Relaxed), self.inner.current_len.load(Ordering::Relaxed), self.inner.blocker.should_theoretically_wait(id == self.inner.current_vec_len.load(Ordering::Relaxed)));
                self.inner.currently_pushing.fetch_add(1, Ordering::Relaxed);
                true
            }
            else {
                self.inner.currently_pushing.fetch_sub(1, Ordering::Relaxed);
                self.inner.blocker.start_wait();
                {
                    let data = self.inner.data.get().as_mut().unwrap();
                    while data.len() <= self.inner.current_len.load(Ordering::Relaxed).max(data.capacity().max(1) - 1) {
                        while self.inner.currently_pushing.load(Ordering::Relaxed) > 0 {
                            
                            thread::sleep(Duration::from_nanos(10))
                        }
                        data.push(MaybeUninit::uninit());
                    }
                    self.inner.current_vec_len.store(data.len(), Ordering::Relaxed);
                }
                self.inner.blocker.stop_waiting();
                self.inner.currently_pushing.fetch_add(1, Ordering::Relaxed);
                true
            }
        }
        
    }
    #[inline(always)]
    /// Safety : Must NOT be used at the same time as `get_unchecked`, `drop_all_elems`, `read_all_elems` or `consume_all_elems`
    pub unsafe fn push(&self, val:T) -> usize {
        unsafe {
            self.inner.currently_pushing.fetch_add(1, Ordering::Relaxed);
            let id = self.inner.current_len.fetch_add(1, Ordering::Relaxed);
            let mut t = self.try_push_with_id(val.clone(), id);
            while t {
                //dbg!("LOOP", self.inner.currently_pushing.load(Ordering::Acquire));
                t = self.try_push_with_id(val.clone(), id);
            }
            id
        }
    }
    #[inline(always)]

    /// Safety : Must NOT be used at the same time as `push`, `drop_all_elems`, `get_unchecked` or `consume_all_elems`
    pub unsafe fn read_all_elems<F:FnMut(&T)>(&self, f:&mut F) {
        unsafe {
            let data = self.inner.data.get().as_ref().unwrap();
            let len = self.inner.current_len.load(Ordering::Relaxed);
            assert!(len <= data.len());
            for i in 0..len {
                f(data[i].assume_init_ref())
            }
        }
    }
    #[inline(always)]
    /// Safety : Must NOT be used at the same time as `push`, `drop_all_elems`, `read_all_elems` or `get_unchecked`
    pub unsafe fn consume_all_elems<F:FnMut(&mut T)>(&self, f:&mut F) {
        unsafe {
            if !self.inner.currently_consuming.fetch_or(true, Ordering::AcqRel) {
                let data = self.inner.data.get().as_mut().unwrap();
                let len = self.inner.current_len.load(Ordering::Relaxed);
                assert!(len <= data.len());
                for i in 0..len {
                    let d = data[i].assume_init_mut();
                    f(d);
                    data[i].assume_init_drop();
                }
                self.inner.current_len.store(0, Ordering::Relaxed);
                self.inner.currently_consuming.store(false, Ordering::Relaxed);
            }
        }
    }
    #[inline(always)]
    /// Safety : Must NOT be used at the same time as `push`, `drop_all_elems`, `read_all_elems` or `drop_all_elems`
    pub unsafe fn drop_all_elems(&self) {
        unsafe {
            if !self.inner.currently_consuming.fetch_or(true, Ordering::AcqRel) {
                let data = self.inner.data.get().as_mut().unwrap();
                let len = self.inner.current_len.load(Ordering::Relaxed);
                assert!(len <= data.len());
                for i in 0..len {
                    data[i].assume_init_drop();
                }
                self.inner.current_len.store(0, Ordering::Relaxed);
                self.inner.currently_consuming.store(false, Ordering::Relaxed);
            }
        }
    }
}

