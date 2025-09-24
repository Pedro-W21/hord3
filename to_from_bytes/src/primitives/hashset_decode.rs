use std::{collections::HashSet, hash::Hash};

use crate::{primitives::vec_decode::VecDecoder, ByteDecoder, FromBytes, ToBytes};
#[derive(Clone)]
pub struct HashSetDecoder<T:FromBytes> {
    vec:VecDecoder<T>
}

impl<T:ToBytes + FromBytes> ToBytes for HashSet<T> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.len().add_bytes(bytes);
        for elt in self {
            elt.add_bytes(bytes);
        }
    }
    fn get_bytes_size(&self) -> usize {
        let mut total = (usize::BITS / 8) as usize;
        for elt in self.iter() {
            total += elt.get_bytes_size()
        }
        total
    }
}

impl<T:FromBytes + Eq + Hash> FromBytes for HashSet<T> {
    type Decoder = HashSetDecoder<T>;
    fn get_decoder() -> Self::Decoder {
        HashSetDecoder {
            vec:Vec::get_decoder()
        }
    }
}

impl<T:FromBytes + Eq + Hash> ByteDecoder<HashSet<T>> for HashSetDecoder<T> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<HashSet<T>> {
        match <VecDecoder<T> as ByteDecoder<Vec<T>>>::decode_byte(&mut self.vec,bytes, byte) {
            Some(vec) => {
                let mut hashmap = HashSet::with_capacity(vec.len());
                for k in vec {
                    hashmap.insert(k);
                }
                Some(hashmap)
            },
            None => None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(HashSet<T>, usize)> {
        match <VecDecoder<T> as ByteDecoder<Vec<T>>>::decode_slice_borrow(&mut self.vec,bytes, slice_to_decode) {
            Some((vec, bytes_read)) => {
                let mut hashmap = HashSet::with_capacity(vec.len());
                for k in vec {
                    hashmap.insert(k);
                }
                Some((hashmap, bytes_read))
            },
            None => None
        }
    }
}
