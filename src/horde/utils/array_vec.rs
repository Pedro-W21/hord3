use std::{slice::SliceIndex, ops::{Index, IndexMut}};

use to_from_bytes::{FromBytes, ToBytes};
use to_from_bytes_derive::{ToBytes, FromBytes};

#[derive(Clone, ToBytes, FromBytes)]
pub struct ArrayVec<T: Sized + Clone + FromBytes + ToBytes,const N:usize> {
    data: [T ; N],
    len:usize,
}

impl<T: Sized + Clone + FromBytes + ToBytes,const N:usize> ArrayVec<T,N> {
    pub fn new(default:T) -> Self {
        Self { data: [0 ; N].map(|num| {default.clone()}), len: 0 }
    }
    pub fn new_full(default:T) -> Self {
        Self { data: [0 ; N].map(|num| {default.clone()}), len: N }
    }
    pub fn push(&mut self, val:T) {
        if self.len < N {
            self.data[self.len] = val;
            self.len += 1;
        }
        else {
            panic!("Tried to push into a full array.")
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, at:usize) -> Option<&T> {
        self.data.get(at)
    }
    pub fn get_mut(&mut self, at:usize) -> Option<&mut T> {
        self.data.get_mut(at)
    }
    pub fn remove(&mut self, at:usize) -> T {
        let val = self[at].clone();
        self.len -= 1;
        for i in at..self.len {
            self[i] = self[i + 1].clone();
        }
        val
    }
    
}

impl<T:Sized + PartialEq + Clone + FromBytes + ToBytes, const N:usize> ArrayVec<T, N> {
    pub fn contains(&self, value:&T) -> bool {
        for val in &self.data[0..self.len] {
            if value == val {
                return true
            }
        }
        false
    }
} 

impl<T:Sized + Clone + FromBytes + ToBytes, const N:usize> Index<usize> for ArrayVec<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        match self.get(index) {
            Some(data) => data,
            None => panic!("Index is {} but len is {}", index, N)
        }
    }
}

impl<T:Sized + Clone + FromBytes + ToBytes, const N:usize> IndexMut<usize> for ArrayVec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self.get_mut(index) {
            Some(data) => data,
            None => panic!("Index is {} but len is {}", index, N)
        }
    }
}

