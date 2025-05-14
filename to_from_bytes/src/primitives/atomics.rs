use std::sync::{atomic::{AtomicU16, Ordering, AtomicUsize, AtomicBool}, Arc};

use atomic_float::AtomicF32;

use crate::{ToBytes, FromBytes, ByteDecoder};

use super::{integers::IntegerDecoder, floats::FloatDecoder, BoolDecoder};

impl ToBytes for Arc<AtomicU16> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.load(Ordering::Relaxed).add_bytes(bytes)
    }
    fn get_bytes_size(&self) -> usize {
        self.load(Ordering::Relaxed).get_bytes_size()
    }
}

impl ByteDecoder<Arc<AtomicU16>> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Arc<AtomicU16>> {
        match <Self as ByteDecoder<u16>>::decode_byte(self, bytes, byte) {
            Some(val) => Some(Arc::new(AtomicU16::new(val))),
            None => None
        }
    }
}

impl FromBytes for Arc<AtomicU16> {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {counter:2}
    }
}


impl ToBytes for Arc<AtomicUsize> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.load(Ordering::Relaxed).add_bytes(bytes)
    }
    fn get_bytes_size(&self) -> usize {
        self.load(Ordering::Relaxed).get_bytes_size()
    }
}

impl ByteDecoder<Arc<AtomicUsize>> for IntegerDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Arc<AtomicUsize>> {
        match <Self as ByteDecoder<usize>>::decode_byte(self, bytes, byte) {
            Some(val) => Some(Arc::new(AtomicUsize::new(val))),
            None => None
        }
    }
}

impl FromBytes for Arc<AtomicUsize> {
    type Decoder = IntegerDecoder;
    fn get_decoder() -> Self::Decoder {
        IntegerDecoder {counter:(usize::BITS / 8) as u8}
    }
}



impl ToBytes for Arc<AtomicBool> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.load(Ordering::Relaxed).add_bytes(bytes)
    }
    fn get_bytes_size(&self) -> usize {
        self.load(Ordering::Relaxed).get_bytes_size()
    }
}

impl ByteDecoder<Arc<AtomicBool>> for BoolDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Arc<AtomicBool>> {
        match <Self as ByteDecoder<bool>>::decode_byte(self, bytes, byte) {
            Some(val) => Some(Arc::new(AtomicBool::new(val))),
            None => None
        }
    }
}

impl FromBytes for Arc<AtomicBool> {
    type Decoder = BoolDecoder;
    fn get_decoder() -> Self::Decoder {
        BoolDecoder { value:0}
    }
}



impl ToBytes for Arc<AtomicF32> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.load(Ordering::Relaxed).add_bytes(bytes)
    }
    fn get_bytes_size(&self) -> usize {
        self.load(Ordering::Relaxed).get_bytes_size()
    }
}

impl ByteDecoder<Arc<AtomicF32>> for FloatDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Arc<AtomicF32>> {
        match <Self as ByteDecoder<f32>>::decode_byte(self, bytes, byte) {
            Some(val) => Some(Arc::new(AtomicF32::new(val))),
            None => None
        }
    }
}

impl FromBytes for Arc<AtomicF32> {
    type Decoder = FloatDecoder;
    fn get_decoder() -> Self::Decoder {
        FloatDecoder {counter:4}
    }
}

impl<T:ToBytes> ToBytes for Arc<T> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.as_ref().add_bytes(bytes)
    }
    fn get_bytes_size(&self) -> usize {
        self.as_ref().get_bytes_size()
    }
}

impl<T:FromBytes> ByteDecoder<Arc<T>> for T::Decoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<Arc<T>> {
        match <Self as ByteDecoder<T>>::decode_byte(self, bytes, byte) {
            Some(val) => Some(Arc::new(val)),
            None => None
        }
    }
}

impl<T:FromBytes> FromBytes for Arc<T> {
    type Decoder = T::Decoder;
    fn get_decoder() -> Self::Decoder {
        T::get_decoder()
    }
}