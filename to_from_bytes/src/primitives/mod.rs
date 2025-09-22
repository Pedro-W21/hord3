use crate::{ToBytes, FromBytes, ByteDecoder};

pub mod integers;
pub mod floats;
pub mod vec_decode;
pub mod strings;
pub mod vecdeque_decode;
pub mod tuples;
pub mod std_enums;
pub mod array_decode;
pub mod hashset_decode;
pub mod atomics;
pub mod hashmap;

impl ToBytes for bool {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        bytes.push(if *self {1} else {0});
    }
    fn get_bytes_size(&self) -> usize {
        1
    }
}
#[derive(Clone)]
pub struct BoolDecoder {
    pub value:u8,
}

impl ByteDecoder<bool> for BoolDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<bool> {
        if byte == 1 {
            Some(true)
        }
        else {
            Some(false)
        }
    }
}

impl FromBytes for bool {
    type Decoder = BoolDecoder;
    fn get_decoder() -> Self::Decoder {
        BoolDecoder {value:0}
    }
}