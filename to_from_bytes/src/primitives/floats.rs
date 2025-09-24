use crate::{ByteDecoder, FromBytes, ToBytes};
#[derive(Clone)]
pub struct FloatDecoder {
    pub counter:u8,
}

impl ByteDecoder<f64> for FloatDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<f64> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 8];
            for i in 0..8 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(f64::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(f64, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 8 {
            Some((f64::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3], slice_to_decode[4], slice_to_decode[5], slice_to_decode[6], slice_to_decode[7]]), 8))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = f64::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3], bytes[4],bytes[5],bytes[6],bytes[7]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
    
}

impl FromBytes for f64 {
    type Decoder = FloatDecoder;
    fn get_decoder() -> Self::Decoder {
        FloatDecoder {
            counter:8
        }
    }
}

impl ToBytes for f64 {
    fn get_bytes_size(&self) -> usize {
        8
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}



impl ByteDecoder<f32> for FloatDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<f32> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 4];
            for i in 0..4 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(f32::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(f32, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 4 {
            Some((f32::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3]]), 4))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = f32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for f32 {
    type Decoder = FloatDecoder;
    fn get_decoder() -> Self::Decoder {
        FloatDecoder {
            counter:4
        }
    }
}

impl ToBytes for f32 {
    fn get_bytes_size(&self) -> usize {
        4
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}