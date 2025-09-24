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
                return Some(None);
            }
            else {
                *self = Some(T::get_decoder());
            },
            Self::Some(decoder) => {
                match decoder.decode_byte(bytes, byte) {
                    Some(decoded) => {
                        return Some(Some(decoded))
                    },
                    None => (),
                }
            }
        }
        None
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(Option<T>, usize)> {
        for i in 0..slice_to_decode.len() {
            let byte = slice_to_decode[i];
            let out = match self {
                Self::None => if byte == 0 {
                    Some((None, 1))
                }
                else {
                    *self = Some(T::get_decoder());
                    None
                },
                Self::Some(decoder) => {
                    match decoder.decode_slice_borrow(bytes, &slice_to_decode[i..]) {
                        Some((decoded, bytes_read)) => {
                            Some((Some(decoded), bytes_read))
                        },
                        None => None,
                    }
                }
            };
            match out {
                None => (),
                Some((decoded, bytes_read)) => return Some((decoded, i + bytes_read))
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