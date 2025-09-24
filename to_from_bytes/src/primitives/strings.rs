use crate::{primitives::vec_decode::VecDecoder, ByteDecoder, FromBytes, ToBytes};

impl ToBytes for String {
    fn get_bytes_size(&self) -> usize {
        (usize::BITS / 8) as usize + self.len()
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.len().add_bytes(bytes);
        for byte in self.as_bytes() {
            bytes.push(*byte)
        }
    }
}
#[derive(Clone)]
pub struct StringDecoder {
    vec:VecDecoder<u8>
}

impl FromBytes for String {
    type Decoder = StringDecoder;
    fn get_decoder() -> Self::Decoder {
        StringDecoder {
            vec:Vec::get_decoder()
        }
    }
}

impl ByteDecoder<String> for StringDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<String> {
        match <VecDecoder<u8> as ByteDecoder<Vec<u8>>>::decode_byte(&mut self.vec,bytes, byte) {
            Some(vec) => {
                Some(String::from_utf8(vec).unwrap())
            },
            None => None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(String, usize)> {
        match <VecDecoder<u8> as ByteDecoder<Vec<u8>>>::decode_slice_borrow(&mut self.vec,bytes, slice_to_decode) {
            Some((vec, bytes_read)) => {
                Some((String::from_utf8(vec).unwrap(), bytes_read))
            },
            None => None
        }
    }
}