use std::collections::VecDeque;

use crate::{ToBytes, FromBytes, ByteDecoder};
#[derive(Clone)]
pub struct VecDequeDecoder<T:FromBytes> {
    counter:u32,
    got_len:bool,
    element_decoder:T::Decoder,
    elements:VecDeque<T>
}

impl<T:ToBytes> ToBytes for VecDeque<T> {
    fn get_bytes_size(&self) -> usize {
        if self.len() > 0 {
            let mut total = 4;
            for i in 0..self.len() {
                total += self[i].get_bytes_size();
            }
            total
        }
        else {
            4
        }
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = (self.len() as u32).to_le_bytes();
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
            counter:4,
            got_len:false,
            element_decoder:T::get_decoder(),
            elements:VecDeque::new(),
        }
    }
}

impl<T:FromBytes> ByteDecoder<VecDeque<T>> for VecDequeDecoder<T> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<VecDeque<T>> {
        
        if self.got_len {
            match self.element_decoder.decode_byte(bytes, byte) {
                Some(element) => {
                    self.counter -= 1;
                    self.elements.push_back(element);
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
                self.elements = VecDeque::with_capacity(self.counter as usize);
                self.got_len = true;
                bytes.clear();
            }
            if self.counter == 0 {
                return Some(VecDeque::new())
            }
            None
        }
        
    }
}