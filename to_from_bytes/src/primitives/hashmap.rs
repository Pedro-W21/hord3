use std::{collections::HashMap, hash::Hash};

use crate::{FromBytes, ToBytes, ByteDecoder};

use super::vec_decode::VecDecoder;



impl<K:ToBytes, V:ToBytes> ToBytes for HashMap<K, V> {
    fn add_bytes(&self, bytes:&mut Vec<u8>) {
        self.len().add_bytes(bytes);
        for (k,v) in self.iter() {
            k.add_bytes(bytes);
            v.add_bytes(bytes);
        }
    }
    fn get_bytes_size(&self) -> usize {
        let mut total = 8;
        for (k,v) in self.iter() {
            total += k.get_bytes_size();
            total += v.get_bytes_size();
        }
        total
    }
}
#[derive(Clone)]
pub struct HashMapDecode<K:FromBytes + Eq + Hash, V:FromBytes> {
    vec:VecDecoder<(K, V)>
}

impl<K:FromBytes + Eq + Hash, V:FromBytes> ByteDecoder<HashMap<K,V>> for HashMapDecode<K, V> {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<HashMap<K,V>> {
        match <VecDecoder<(K,V)> as ByteDecoder<Vec<(K,V)>>>::decode_byte(&mut self.vec,bytes, byte) {
            Some(vec) => {
                let mut hashmap = HashMap::with_capacity(vec.len());
                for (k,v) in vec {
                    hashmap.insert(k, v);
                }
                Some(hashmap)
            },
            None => None
        }
    }
}

impl<K:FromBytes + Eq + Hash, V:FromBytes> FromBytes for HashMap<K, V> {
    type Decoder = HashMapDecode<K,V>;
    fn get_decoder() -> Self::Decoder {
        HashMapDecode {
            vec:Vec::get_decoder()
        }
    }
}