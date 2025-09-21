use crate::{ToBytes, FromBytes, ByteDecoder};

impl<T:ToBytes> ToBytes for Option<T> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        match self {
            Self::Some(data) => {
                bytes.push(1);
                data.add_bytes(bytes);
            },
            Self::None => bytes.push(0),
        }
    }
    fn get_bytes_size(&self) -> usize {
        match self {
            Self::Some(data) => 1 + data.get_bytes_size(),
            Self::None => 1,
        }
    }
}

impl<T:FromBytes> ByteDecoder<Option<T>> for Option<T::Decoder> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Option<T>> {
        match self {
            Self::None => if byte == 0 {
                if bytes.len() > 0 {
                    panic!("bytes leftover");
                }
                return Some(None);
            }
            else {
                if bytes.len() > 0 {
                    panic!("bytes leftover");
                }
                *self = Some(T::get_decoder());
            },
            Self::Some(decoder) => {
                match decoder.decode_byte(bytes, byte) {
                    Some(decoded) => return Some(Some(decoded)),
                    None => (),
                }
            }
        }
        None
    }
}

impl<T:FromBytes> FromBytes for Option<T> {
    type Decoder = Option<T::Decoder>;
    fn get_decoder() -> Self::Decoder {
        None
    }
}