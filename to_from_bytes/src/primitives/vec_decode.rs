use crate::{ToBytes, FromBytes, ByteDecoder};
#[derive(Clone)]
pub struct VecDecoder<T:FromBytes> {
    counter:usize,
    got_len:bool,
    element_decoder:T::Decoder,
    elements:Vec<T>
}

impl<T:ToBytes> ToBytes for Vec<T> {
    fn get_bytes_size(&self) -> usize {
        if self.len() > 0 {
            let mut total = (usize::BITS / 8) as usize;
            for i in 0..self.len() {
                total += self[i].get_bytes_size();
            }
            total
        }
        else {
            (usize::BITS / 8) as usize
        }
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.len().add_bytes(bytes);
        for elt in self {
            elt.add_bytes(bytes);
        }
    }
}

impl<T:FromBytes> FromBytes for Vec<T> {
    type Decoder = VecDecoder<T>;
    fn get_decoder() -> Self::Decoder {
        VecDecoder {
            counter:(usize::BITS / 8) as usize,
            got_len:false,
            element_decoder:T::get_decoder(),
            elements:Vec::new(),
        }
    }
}

impl<T:FromBytes> ByteDecoder<Vec<T>> for VecDecoder<T> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Vec<T>> {
        if self.got_len {
            
            match self.element_decoder.decode_byte(bytes, byte) {
                Some(element) => {
                    self.counter -= 1;
                    self.elements.push(element);
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
                self.counter = usize::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3],bytes[4], bytes[5], bytes[6], bytes[7]]);
                dbg!(self.counter);
                self.elements = Vec::with_capacity(self.counter as usize);
                
                self.got_len = true;
                bytes.clear();
            }
            if self.counter == 0 {
                bytes.clear();
                return Some(Vec::new())
            }
            None
        }
    }
}