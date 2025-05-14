use crate::{ToBytes, FromBytes, ByteDecoder};
use std::mem;
use std::ptr;

impl<T:ToBytes, const SIZE:usize> ToBytes for [T ; SIZE] {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        for elt in self {
            elt.add_bytes(bytes);
        }
    }
    fn get_bytes_size(&self) -> usize {
        let mut total = 0;
        
        for elt in self {
            total += elt.get_bytes_size();
        }
        total
    }
}

#[derive(Clone)]
pub struct ArrayDecoder<T:FromBytes, const SIZE:usize> {
    decoder:T::Decoder,
    index:usize,
    array:[Option<T> ; SIZE],
}

macro_rules! make_array_from_other_array {
    ($n:expr, $array:expr) => {{
        let mut items: [_ ; $n] = mem::uninitialized();
        for (i, place) in items.iter_mut().enumerate() {
            ptr::write(place, $array[i].clone().unwrap());
        }
        items
    }};
}

impl<T:FromBytes, const SIZE:usize> ByteDecoder<[T ; SIZE]> for ArrayDecoder<T, SIZE> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<[T ; SIZE]> {
        match self.decoder.decode_byte(bytes, byte) {
            Some(data) => {self.array[self.index] = Some(data); self.index += 1; self.decoder = T::get_decoder(); bytes.clear();}
            None => (),
        }
        if self.index == SIZE {
            let array_out:[T ; SIZE] = unsafe {make_array_from_other_array!(SIZE, &self.array)};
            bytes.clear();
            Some(array_out)
        }
        else {
            None
        }
    }
}

macro_rules! make_none_array {
    ($n:expr) => {{
        let mut items: [_ ; $n] = mem::uninitialized();
        for place in items.iter_mut() {
            ptr::write(place, None);
        }
        items
    }};
}

impl<T:FromBytes, const SIZE:usize> FromBytes for [T ; SIZE] {
    type Decoder = ArrayDecoder<T, SIZE>;
    fn get_decoder() -> Self::Decoder {
        ArrayDecoder {
            decoder:T::get_decoder(),
            index:0,
            array:unsafe {make_none_array!(SIZE)}
        }
    }
}