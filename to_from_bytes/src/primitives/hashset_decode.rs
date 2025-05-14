use std::{collections::HashSet, hash::Hash};

use crate::{ToBytes, FromBytes, ByteDecoder};
#[derive(Clone)]
pub struct HashSetDecoder<T:FromBytes> {
    counter:u32,
    got_len:bool,
    element_decoder:T::Decoder,
    elements:HashSet<T>
}

impl<T:ToBytes + FromBytes> ToBytes for HashSet<T> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        (self.len() as u32).add_bytes(bytes);
        for elt in self {
            elt.add_bytes(bytes);
        }
    }
    fn get_bytes_size(&self) -> usize {
        let mut total = 4;
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
            counter:4,
            got_len:false,
            element_decoder:T::get_decoder(),
            elements:HashSet::new(),
        }
    }
}

impl<T:FromBytes + Eq + Hash> ByteDecoder<HashSet<T>> for HashSetDecoder<T> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<HashSet<T>> {
        if self.got_len {
            
            match self.element_decoder.decode_byte(bytes, byte) {
                Some(element) => {
                    self.counter -= 1;
                    self.elements.insert(element);
                    self.element_decoder = T::get_decoder();
                    bytes.clear();
                },
                None => ()
            }
            if self.counter == 0 {
                bytes.clear();
                Some(self.elements.clone())
            }
            else {
                
                None
            }
        }
        else {
            self.counter -= 1;
            bytes.push(byte);
            if self.counter == 0 {
                self.counter = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                self.elements = HashSet::with_capacity(self.counter as usize);
                self.got_len = true;
                bytes.clear();
            }
            if self.counter == 0 {
                return Some(HashSet::new())
            }
            None
        }
    }
}
