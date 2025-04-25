use std::{cell::SyncUnsafeCell, mem::MaybeUninit, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc}};

use threading_utils::utils::do_blocker::DoBlocker;

pub struct LAMPMCVec<T:Clone + Sync + Send> {
    inner:Arc<InnerMPMCVec<T>>,
    local_actual_len:usize,
}

pub struct InnerMPMCVec<T:Clone + Sync + Send> {
    currently_consuming:AtomicBool,
    current_len:AtomicUsize,
    data:SyncUnsafeCell<Vec<MaybeUninit<T>>>,
}

impl<T:Clone + Sync + Send> LAMPMCVec<T> {
    pub fn len(&self) -> usize {
        unsafe {
            self.inner.data.get().as_mut().unwrap_unchecked().len()
        }
        
    }
    pub fn new(capacity:usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        for i in 0..capacity {
            data.push(MaybeUninit::uninit());
        }
        Self { inner: Arc::new(InnerMPMCVec {currently_consuming:AtomicBool::new(false), current_len:AtomicUsize::new(0), data:SyncUnsafeCell::new(data)}), local_actual_len: capacity}
    }
    pub unsafe fn push(&self, value:T) -> Result<usize, ()> {
        let index = self.inner.current_len.fetch_add(1, Ordering::Relaxed);

        let data = self.inner.data.get().as_mut().unwrap_unchecked();
        if index < data.len() {
            *data.get_unchecked_mut(index) = MaybeUninit::new(value);
            Ok(index)
        }
        else {
            Err(())
        }
    }
    pub unsafe fn get_unchecked(&self, at:usize) -> &T {
        self.inner.data.get().as_ref().unwrap_unchecked().get_unchecked(at).assume_init_ref()
    }
    pub unsafe fn consume_all_elems<F:FnMut(&mut T)>(&self, f:&mut F) {
        if !self.inner.currently_consuming.fetch_or(true, Ordering::AcqRel) {
            let data = self.inner.data.get().as_mut().unwrap();
            let len = self.inner.current_len.load(Ordering::Relaxed);
            //assert!(len <= data.len());
            for i in 0..len.min(data.len()) {
                let d = data[i].assume_init_mut();
                f(d);
                data[i].assume_init_drop();
            }
            //dbg!(self.inner.current_len.load(Ordering::Relaxed));
            self.resize_if_needed();
            self.inner.current_len.store(0, Ordering::Relaxed);
            self.inner.currently_consuming.store(false, Ordering::Relaxed);
        }
    }
    pub unsafe fn resize_if_needed(&self) {
        let data = self.inner.data.get().as_mut().unwrap();
        if self.inner.current_len.load(Ordering::Relaxed) >= data.len() {
            *data = Vec::with_capacity(self.inner.current_len.load(Ordering::Relaxed) * 4);
            for _i in 0..data.capacity() {
                data.push(MaybeUninit::uninit());
            }

        }
    }
}