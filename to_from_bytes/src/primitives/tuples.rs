use crate::{ToBytes, ByteDecoder, FromBytes};
#[derive(Clone)]
pub enum EitherOr<T,U> {
    First(T),
    Second(U)
}

impl<T:ToBytes, U:ToBytes, V:ToBytes> ToBytes for (T,U,V) {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.0.add_bytes(bytes);
        self.1.add_bytes(bytes);
        self.2.add_bytes(bytes);
    }
    fn get_bytes_size(&self) -> usize {
        self.0.get_bytes_size() + self.1.get_bytes_size() + self.2.get_bytes_size()
    }
}

impl<T:FromBytes, U:FromBytes, V:FromBytes> ByteDecoder<(T,U,V)> for (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>,EitherOr<V::Decoder, V>) {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<(T,U,V)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                Some(decoded) => {self.0 = EitherOr::Second(decoded); bytes.clear();},
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                        Some(decoded) => {self.1 = EitherOr::Second(decoded); bytes.clear();},
                        None => ()
                    },
                    EitherOr::Second(decoded_1) => {
                        match &mut self.2 {
                            EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                                Some(decoded_2) => {bytes.clear();return Some((decoded_0.clone(), decoded_1.clone(), decoded_2))},
                                None => ()
                            },
                            _ => ()
                        }
                    }
                }
            }
        }
        None
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<((T,U,V), usize)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                Some((decoded, bytes_read)) => {
                    self.0 = EitherOr::Second(decoded);
                    bytes.clear();
                    match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                        Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                        None => ()
                    }
                },
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                        Some((decoded, bytes_read)) => {
                            self.1 = EitherOr::Second(decoded);
                            bytes.clear();
                            match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                                Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                                None => ()
                            }
                        },
                        None => ()
                    },
                    EitherOr::Second(decoded_1) => {
                        match &mut self.2 {
                            EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                                Some((decoded, bytes_read)) => {
                                    bytes.clear();
                                    return Some(((decoded_0.clone(), decoded_1.clone(), decoded), bytes_read))
                                },
                                None => ()
                            },
                            _ => ()
                        }
                    }
                }
            }
        }
        None
    }
}

impl<T:FromBytes, U:FromBytes, V:FromBytes> FromBytes for (T,U,V) {
    type Decoder = (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>,EitherOr<V::Decoder, V>);
    fn get_decoder() -> Self::Decoder {
        (
            EitherOr::First(T::get_decoder()),
            EitherOr::First(U::get_decoder()),
            EitherOr::First(V::get_decoder()),
        )
    }
}

impl<T:ToBytes, U:ToBytes> ToBytes for (T,U) {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.0.add_bytes(bytes);
        self.1.add_bytes(bytes);
    }
    fn get_bytes_size(&self) -> usize {
        self.0.get_bytes_size() + self.1.get_bytes_size()
    }
}

impl<T:FromBytes, U:FromBytes> ByteDecoder<(T,U)> for (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>) {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<(T,U)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                Some(decoded) => {bytes.clear();self.0 = EitherOr::Second(decoded);},
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                        Some(decoded_1) => {bytes.clear();return Some((decoded_0.clone(), decoded_1));},
                        None => ()
                    },
                    _ => ()
                }
            }
        }
        None
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<((T,U), usize)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                Some((decoded, bytes_read)) => {
                    self.0 = EitherOr::Second(decoded);
                    bytes.clear();
                    match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                        Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                        None => ()
                    }
                },
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                        Some((decoded, bytes_read)) => {
                            bytes.clear();
                            return Some(((decoded_0.clone(), decoded), bytes_read))
                        },
                        None => ()
                    },
                    _ => ()
                }
            }
        }
        None
    }
}

impl<T:FromBytes, U:FromBytes> FromBytes for (T,U) {
    type Decoder = (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>);
    fn get_decoder() -> Self::Decoder {
        (
            EitherOr::First(T::get_decoder()),
            EitherOr::First(U::get_decoder()),
        )
    }
}


impl<T:ToBytes, U:ToBytes, V:ToBytes, W:ToBytes> ToBytes for (T,U,V,W) {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.0.add_bytes(bytes);
        self.1.add_bytes(bytes);
        self.2.add_bytes(bytes);
        self.3.add_bytes(bytes);
    }
    fn get_bytes_size(&self) -> usize {
        self.0.get_bytes_size() + self.1.get_bytes_size() + self.2.get_bytes_size() + self.3.get_bytes_size()
    }
}

impl<T:FromBytes, U:FromBytes, V:FromBytes, W:FromBytes> ByteDecoder<(T,U,V,W)> for (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>,EitherOr<V::Decoder, V>,EitherOr<W::Decoder, W>) {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<(T,U,V,W)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                Some(decoded) => {self.0 = EitherOr::Second(decoded); bytes.clear();},
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                        Some(decoded) => {self.1 = EitherOr::Second(decoded); bytes.clear();},
                        None => ()
                    },
                    EitherOr::Second(decoded_1) => {
                        match &mut self.2 {
                            EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                                Some(decoded_2) => {self.2 = EitherOr::Second(decoded_2);bytes.clear();},
                                None => ()
                            },
                            EitherOr::Second(decoded_2) => {
                                match &mut self.3 {
                                    EitherOr::First(decoder) => match decoder.decode_byte(bytes, byte) {
                                        Some(decoded_3) => {bytes.clear(); return Some((decoded_0.clone(), decoded_1.clone(), decoded_2.clone(), decoded_3))},
                                        None => ()
                                    }
                                    _ => ()
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<((T,U,V,W), usize)> {
        match &mut self.0 {
            EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                Some((decoded, bytes_read)) => {
                    self.0 = EitherOr::Second(decoded);
                    bytes.clear();
                    match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                        Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                        None => ()
                    }
                },
                None => ()
            },
            EitherOr::Second(decoded_0) => {
                match &mut self.1 {
                    EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                        Some((decoded, bytes_read)) => {
                            self.1 = EitherOr::Second(decoded);
                            bytes.clear();
                            match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                                Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                                None => ()
                            }
                        },
                        None => ()
                    },
                    EitherOr::Second(decoded_1) => {
                        match &mut self.2 {
                            EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                                Some((decoded, bytes_read)) => {
                                    self.2 = EitherOr::Second(decoded);
                                    bytes.clear();
                                    match self.decode_slice_borrow(bytes, &slice_to_decode[bytes_read..]) {
                                        Some((full, total_read)) => return Some((full, total_read + bytes_read)),
                                        None => ()
                                    }
                                },
                                None => ()
                            },
                            EitherOr::Second(decoded_2) => {
                                match &mut self.3 {
                                    EitherOr::First(decoder) => match decoder.decode_slice_borrow(bytes, slice_to_decode) {
                                        Some((decoded, bytes_read)) => {
                                            bytes.clear();
                                            return Some(((decoded_0.clone(), decoded_1.clone(), decoded_2.clone(), decoded), bytes_read))
                                            
                                        },
                                        None => ()
                                    },
                                    _ => ()
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl<T:FromBytes, U:FromBytes, V:FromBytes, W:FromBytes> FromBytes for (T,U,V,W) {
    type Decoder = (EitherOr<T::Decoder, T>,EitherOr<U::Decoder, U>,EitherOr<V::Decoder, V>,EitherOr<W::Decoder, W>);
    fn get_decoder() -> Self::Decoder {
        (
            EitherOr::First(T::get_decoder()),
            EitherOr::First(U::get_decoder()),
            EitherOr::First(V::get_decoder()),
            EitherOr::First(W::get_decoder()),
        )
    }
}