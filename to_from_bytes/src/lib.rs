#![feature(sync_unsafe_cell)]
use std::{fs::File, io::{ErrorKind, Read, Write}, net::TcpStream, path::PathBuf};



pub mod primitives;
pub trait ToBytes:Sized + Clone {
    fn add_bytes(&self, bytes:&mut Vec<u8>);
    fn get_bytes_size(&self) -> usize;
}

pub trait ByteDecoder<T:FromBytes>:Clone {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<T>; //a
    fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(T, usize)> {
        for i in 0..slice_to_decode.len() {
            match self.decode_byte(bytes, slice_to_decode[i]) {
                Some(decoded) => return Some((decoded, i + 1)),
                None => ()
            }
        }
        None
    }
}

pub trait ByteDecoderUtilities<T:FromBytes>:ByteDecoder<T> {
    fn decode_bytes(&mut self, bytes:&mut Vec<u8>, bytes_to_decode:Vec<u8>) -> Option<T>;
    fn decode_bytes_borrow(&mut self, bytes:&mut Vec<u8>, bytes_to_decode:&mut Vec<u8>) -> Option<T>;
    fn decode_multiple_from_slice(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Vec<T>;
    
}

impl<T:FromBytes<Decoder = BD>, BD:ByteDecoder<T>> ByteDecoderUtilities<T> for BD {
    fn decode_bytes(&mut self, bytes:&mut Vec<u8>, bytes_to_decode:Vec<u8>) -> Option<T> {
        for byte in bytes_to_decode {
            match self.decode_byte(bytes, byte) {
                Some(val) => return Some(val),
                None => ()
            }
        }
        None
    }
    fn decode_bytes_borrow(&mut self, bytes:&mut Vec<u8>, bytes_to_decode:&mut Vec<u8>) -> Option<T> {
        let mut found_decode = false;
        let mut decoded = None;
        bytes_to_decode.retain(|byte| {
            if !found_decode {
                match self.decode_byte(bytes, *byte) {
                    // Pas bon là, on peut effacer alors que y'a d'autres choses dans le vec
                    Some(val) => {decoded = Some(val); found_decode = true},
                    None => ()
                }
            } 
            found_decode
        });
        /* 
        for i in 0..bytes_to_decode.len() {
            match self.decode_byte(bytes, bytes_to_decode[i]) {
                // Pas bon là, on peut effacer alors que y'a d'autres choses dans le vec
                Some(val) => {bytes_to_decode.clear(); return Some(val)},
                None => ()
            }
        }
        
        bytes_to_decode.clear(); // Bon ici
        */
        decoded
    }
    fn decode_multiple_from_slice(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Vec<T> {
        let mut total_decoded = Vec::with_capacity(2);
        let mut end_of_decode = 0;
        while end_of_decode < slice_to_decode.len() {
            match self.decode_slice_borrow(bytes, &slice_to_decode[end_of_decode..]) {
                Some((decoded, bytes_read)) => {
                    //println!("decoded after {} bytes read", bytes_read);
                    end_of_decode += bytes_read;
                    total_decoded.push(decoded);
                    *self = T::get_decoder();
                },
                None => end_of_decode = slice_to_decode.len()
            }
        }

        total_decoded
    }
    
}

pub trait FromBytes:ToBytes {
    type Decoder:ByteDecoder<Self>;
    fn get_decoder() -> Self::Decoder;
}

pub trait ToBytesUtilities {
    fn to_bytes(&self) -> Vec<u8>;
}

impl<AB:ToBytes> ToBytesUtilities for AB {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.get_bytes_size());
        self.add_bytes(&mut bytes);
        bytes
    }
}


pub fn type_from_file<T:FromBytes>(path:PathBuf) -> Result<T, ()> {
    if path.exists() {
        match File::open(path) {
            Ok(mut file) => {
                let mut bytes_to_decode = match file.metadata() {
                    Ok(metadata) => Vec::with_capacity(metadata.len() as usize),
                    Err(_) => Vec::new()
                };
                file.read_to_end(&mut bytes_to_decode);
                
                let mut bytes = Vec::with_capacity(1_000_000);
                match T::get_decoder().decode_bytes(&mut bytes, bytes_to_decode) {
                    Some(save) => Ok(save),
                    None => Err(())
                }

            },
            Err(_) => Err(())
        }
    }
    else {
        Err(())
    }
}

pub fn save_type<T:ToBytes>(path:PathBuf, data:T) -> bool { // succeeded
    let mut succeeded = false;
    let mut test_path = path.clone();
    if test_path.pop() && !test_path.to_str().unwrap().is_empty() {
        if test_path.exists() {
            match path.file_name() {
                Some(name) => {
                    if !name.is_empty() {
                        match File::create(path) {
                            Ok(mut file) => {
                                let mut bytes = Vec::with_capacity(1_000_000);
                                data.add_bytes(&mut bytes);
                                match file.write_all(&bytes) {
                                    Ok(()) => succeeded = true,
                                    Err(_) => (),
                                }
                            },
                            Err(_) => succeeded= false,
                        }
                    }
                },
                None => ()
            }
        }
    }
    else {
        match path.to_str() {
            Some(name) => {
                if !name.is_empty() {
                    match File::create(path) {
                        Ok(mut file) => {
                            let mut bytes = Vec::with_capacity(1_000_000);
                            data.add_bytes(&mut bytes);
                            match file.write_all(&bytes) {
                                Ok(()) => succeeded = true,
                                Err(_) => (),
                            }
                        },
                        Err(_) => succeeded= false,
                    }
                }
            },
            None => ()
        }
    }

    succeeded
}

pub fn decode_from_tcp<const BLOCKING:bool, T:FromBytes + ToBytes>(decoder:&mut T::Decoder, tcp:&mut TcpStream, tcp_buffer:&mut Vec<u8>, decoding_bytes:&mut Vec<u8>) -> Vec<T> {
    let mut all_decoded = Vec::with_capacity(1);
    if !BLOCKING {
        loop {
            match tcp.read(tcp_buffer) {
                Ok(bytes_read) => {
                    // println!("Read {:?}", &tcp_buffer[..bytes_read]);
                    all_decoded.append(&mut decoder.decode_multiple_from_slice(decoding_bytes, &tcp_buffer[..bytes_read]));
                },
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) => panic!("Error while decoding from tcp {}", error),
            }
        }
    }
    //println!("Decoded : {}", all_decoded.len());
    all_decoded
}