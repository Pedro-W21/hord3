use vulkano::{DeviceSize, buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}};

use crate::defaults::default_rendering::mantle::meshes::MemoryAllocator;

pub struct BufferAllocator<T:BufferContents + Copy> {
    available:Vec<AvailableMemory>,
    buffer:Subbuffer<[T]>,
    allocator:MemoryAllocator,
}

impl<T:BufferContents + Copy> BufferAllocator<T> {
    pub fn new(capacity:usize, allocator:MemoryAllocator, buffer_usage:BufferUsage) -> Self {
        let buffer = Buffer::new_slice::<T>(
            allocator.clone(),
            BufferCreateInfo {
                usage: buffer_usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            capacity as DeviceSize,
        ).expect("Couldn't allocate global buffer");
        Self { available: vec![AvailableMemory {start:0, length:capacity}], buffer: buffer, allocator }
    }
    pub fn get_buffer(&self) -> Subbuffer<[T]> {
        self.buffer.clone()
    }
    pub fn allocate(&mut self, demand:AllocationDemand) -> Allocation {
        if demand.max_length == 0 {
            return Allocation { start: 0, length: 0, max_length: 0 }
        }
        let mut alloc = None;
        let mut slot = 0;
        for (i, av) in self.available.iter().enumerate() {
            if demand.max_length <= av.length {
                let start = av.start;
                slot = i;
                alloc = Some(Allocation { start, length: demand.start_length, max_length: demand.max_length });
                break;
            }
        }
        match alloc {
            Some(alloc) => {
                let mut av = self.available.remove(slot);
                if av.length != alloc.max_length {
                    av.start += alloc.max_length;
                    av.length -= alloc.max_length;
                    self.insert_available(av);
                }
                alloc
            }
            None => panic!("No allocation possibility found !")
        }
        
    }
    fn insert_available(&mut self, available:AvailableMemory) -> usize {
        let mut insert_index = 0;
        for i in 0..self.available.len() {
            if self.available[i].length >= available.length {
                insert_index = i;
                break;
            }
        }
        self.available.insert(insert_index, available);
        insert_index
    }
    pub fn free(&mut self, alloc:Allocation) {
        if alloc.max_length > 0 {
            let mut freed = AvailableMemory {start:alloc.start, length:alloc.max_length};
            let inserted = self.insert_available(freed);
        }
        // stuff to fuse available memory spaces, TODO later
        /*let mut fuse_with_prev = None;
        if inserted > 0 {
            let prev = self.available[inserted - 1];
            if prev.start + prev.length == freed.start {
                fuse_with_prev = Some(prev.start)
            }
        }
        if inserted < self.available.len() - 1 {
            let next = self.available[inserted + 1];
            if freed.start + freed.length == next.start {
                freed.length += next.length;
                self.available.remove(inserted + 1);
            }
        }
        if let Some(start) = fuse_with_prev {
            self.available.remove(inserted - 1);
            freed.length += fred.
            freed.start = start;
            
        }*/
    }
    pub fn extend_allocation(&mut self, mut prev:Allocation, new_demand:AllocationDemand) -> Allocation {
        if new_demand.start_length <= prev.max_length && new_demand.max_length <= prev.max_length {
            prev.length = new_demand.start_length;
            prev
        }
        else {
            self.free(prev);
            self.allocate(new_demand)
        }
    }
    pub fn update_allocation(&mut self, alloc:Allocation, new_data:&Vec<T>) {
        if alloc.max_length > 0 {
            let mut writer = self.buffer.write().unwrap();
            writer.get_mut(alloc.start..(alloc.start + alloc.length)).unwrap().copy_from_slice(new_data);
        }
    }
}

#[derive(Clone, Copy)]
pub struct AvailableMemory {
    start:usize,
    length:usize
}

pub struct AllocationDemand {
    start_length:usize,
    max_length:usize,
}

impl AllocationDemand {
    pub fn simple(length:usize) -> Self {
        Self { start_length: length, max_length: length }
    }
    pub fn new(start_length:usize, max_length:usize) -> Self {
        Self { start_length, max_length }
    }
}

#[derive(Clone, Copy)]
pub struct Allocation {
    start:usize,
    length:usize,
    max_length:usize,

}

impl Allocation {
    pub fn get_length(&self) -> usize {
        self.length
    }
    pub fn get_max_length(&self) -> usize {
        self.max_length
    }
    pub fn get_start(&self) -> usize {
        self.start
    }
}