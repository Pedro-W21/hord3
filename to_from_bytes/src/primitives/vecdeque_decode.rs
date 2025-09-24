use std::collections::VecDeque;

use crate::{primitives::vec_decode::VecDecoder, ByteDecoder, FromBytes, ToBytes};
#[derive(Clone)]
pub struct VecDequeDecoder<T:FromBytes> {
    vec:VecDecoder<T>
}

impl<T:ToBytes> ToBytes for VecDeque<T> {
    fn get_bytes_size(&self) -> usize {
        if self.len() > 0 {
            let mut total = 8;
            for i in 0..self.len() {
                total += self[i].get_bytes_size();
            }
            total
        }
        else {
            8
        }
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = (self.len() as u64).to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
        for elt in self {
            elt.add_bytes(bytes);
        }
    }
}

impl<T:FromBytes> FromBytes for VecDeque<T> {
    type Decoder = VecDequeDecoder<T>;
    fn get_decoder() -> Self::Decoder {
        VecDequeDecoder {
            vec:Vec::get_decoder()
        }
    }
}

impl<T:FromBytes> ByteDecoder<VecDeque<T>> for VecDequeDecoder<T> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<VecDeque<T>> {
        match <VecDecoder<T> as ByteDecoder<Vec<T>>>::decode_byte(&mut self.vec,bytes, byte) {
            Some(vec) => {
                let mut hashmap = VecDeque::with_capacity(vec.len());
                for k in vec {
                    hashmap.push_back(k);
                }
                Some(hashmap)
            },
            None => None
        }
        
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(VecDeque<T>, usize)> {
        match <VecDecoder<T> as ByteDecoder<Vec<T>>>::decode_slice_borrow(&mut self.vec,bytes, slice_to_decode) {
            Some((vec, bytes_read)) => {
                let mut hashmap = VecDeque::with_capacity(vec.len());
                for k in vec {
                    hashmap.push_back(k);
                }
                Some((hashmap, bytes_read))
            },
            None => None
        }
    }
}