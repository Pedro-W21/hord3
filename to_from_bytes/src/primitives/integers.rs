use std::sync::Arc;

use crate::{ByteDecoder, FromBytes, ToBytes};
#[derive(Clone)]
pub struct IntegerDecoder {
    pub counter:u8,
}


impl ByteDecoder<usize> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<usize> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; (usize::BITS/8) as usize];
            for i in 0..usize::BITS/8 {
                bytes_out[i as usize] = bytes[i as usize];
            }
            bytes.clear();
            Some(usize::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(usize, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 8 {
            Some((usize::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3], slice_to_decode[4], slice_to_decode[5], slice_to_decode[6], slice_to_decode[7]]), 8))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = usize::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3], bytes[4],bytes[5],bytes[6],bytes[7]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for usize {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:(usize::BITS/8) as u8
        }
    }
}

impl ToBytes for usize {
    fn get_bytes_size(&self) -> usize {
        (usize::BITS/8) as usize
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        for byte in self.to_le_bytes() {
            bytes.push(byte);
        }
    }
}

impl ByteDecoder<isize> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<isize> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; (isize::BITS/8) as usize];
            for i in 0..isize::BITS/8 {
                bytes_out[i as usize] = bytes[i as usize];
            }
            bytes.clear();
            Some(isize::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(isize, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 8 {
            Some((isize::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3], slice_to_decode[4], slice_to_decode[5], slice_to_decode[6], slice_to_decode[7]]), 8))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = isize::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3], bytes[4],bytes[5],bytes[6],bytes[7]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for isize {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:(isize::BITS/8) as u8
        }
    }
}

impl ToBytes for isize {
    fn get_bytes_size(&self) -> usize {
        (isize::BITS/8) as usize
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        for byte in self.to_le_bytes() {
            bytes.push(byte);
        }
    }
}

impl ByteDecoder<u64> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<u64> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 8];
            for i in 0..8 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(u64::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(u64, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 8 {
            Some((u64::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3], slice_to_decode[4], slice_to_decode[5], slice_to_decode[6], slice_to_decode[7]]), 8))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = u64::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3], bytes[4],bytes[5],bytes[6],bytes[7]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for u64 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:8
        }
    }
}

impl ToBytes for u64 {
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



impl ByteDecoder<u32> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<u32> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 4];
            for i in 0..4 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(u32::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(u32, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 4 {
            Some((u32::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3]]), 4))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = u32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for u32 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:4
        }
    }
}

impl ToBytes for u32 {
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



impl ByteDecoder<u16> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<u16> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 2];
            for i in 0..2 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(u16::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(u16, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 2 {
            Some((u16::from_le_bytes([slice_to_decode[0], slice_to_decode[1]]), 2))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = u16::from_le_bytes([bytes[0],bytes[1]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for u16 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:2
        }
    }
}

impl ToBytes for u16 {
    fn get_bytes_size(&self) -> usize {
        2
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}



impl ByteDecoder<u8> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<u8> {
        Some(u8::from_le_bytes([byte]))
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(u8, usize)> {
        if slice_to_decode.len() > 0 {
            Some((u8::from_le_bytes([slice_to_decode[0]]), 1))
        }
        else {
            None
        }
    }
}

impl FromBytes for u8 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:1
        }
    }
}

impl ToBytes for u8 {
    fn get_bytes_size(&self) -> usize {
        1
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}

impl ByteDecoder<i64> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<i64> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 8];
            for i in 0..8 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(i64::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(i64, usize)> {
        if slice_to_decode.len() >= 8 {
            Some((i64::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3], slice_to_decode[4], slice_to_decode[5], slice_to_decode[6], slice_to_decode[7]]), 8))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = i64::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3], bytes[4],bytes[5],bytes[6],bytes[7]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}


impl FromBytes for i64 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:8
        }
    }
}

impl ToBytes for i64 {
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



impl ByteDecoder<i32> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<i32> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 4];
            for i in 0..4 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(i32::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(i32, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 4 {
            Some((i32::from_le_bytes([slice_to_decode[0], slice_to_decode[1], slice_to_decode[2], slice_to_decode[3]]), 4))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = i32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for i32 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:4
        }
    }
}

impl ToBytes for i32 {
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



impl ByteDecoder<i16> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<i16> {
        bytes.push(byte);
        self.counter -= 1;
        
        if self.counter == 0 {
            let mut bytes_out = [0_u8 ; 2];
            for i in 0..2 {
                bytes_out[i] = bytes[i];
            }
            bytes.clear();
            Some(i16::from_le_bytes(bytes_out))
        }
        else {
            None
        }
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(i16, usize)> {
        if bytes.len() == 0 && slice_to_decode.len() >= 2 {
            Some((i16::from_le_bytes([slice_to_decode[0], slice_to_decode[1]]), 2))
        }
        else {
            for i in 0..slice_to_decode.len() {
                bytes.push(slice_to_decode[i]);
                self.counter -= 1;

                if self.counter == 0 {
                    let val = i16::from_le_bytes([bytes[0],bytes[1]]);
                    bytes.clear();
                    return Some((val, i + 1))
                }
            }
            None
        }
    }
}

impl FromBytes for i16 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:2
        }
    }
}

impl ToBytes for i16 {
    fn get_bytes_size(&self) -> usize {
        2
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}



impl ByteDecoder<i8> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<i8> {
        Some(i8::from_le_bytes([byte]))
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(i8, usize)> {
        if slice_to_decode.len() > 0 {
            Some((i8::from_le_bytes([slice_to_decode[0]]), 1))
        }
        else {
            None
        }
    }
}

impl FromBytes for i8 {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {
            counter:1
        }
    }
}

impl ToBytes for i8 {
    fn get_bytes_size(&self) -> usize {
        1
    }
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        let self_bytes = self.to_le_bytes();
        for byte in self_bytes {
            bytes.push(byte);
        }
    }
}