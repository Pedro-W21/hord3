use to_from_bytes::{ToBytes, FromBytes};
use to_from_bytes_derive::{ToBytes, FromBytes};

use super::array_vec::ArrayVec;

#[derive(Clone, Copy, ToBytes, FromBytes, Debug)]
pub struct BitField {
    pub inner:u8,
}

impl Default for BitField {
    fn default() -> Self {
        Self { inner: 0 }
    }
}

impl From<u8> for BitField {
    fn from(value: u8) -> Self {
        Self { inner: value }
    }
}

impl From<[bool ; 8]> for BitField {
    fn from(value: [bool ; 8]) -> Self {
        let mut final_value = 0_u8;
        for val in value.iter().rev() {
            
            final_value <<= 1;
            if *val {
                final_value += 1;
            }
        }
        Self { inner: final_value }
    }
}

impl From<BitField> for [bool ; 8] {
    fn from(value: BitField) -> Self {
        let mut bools = [false ; 8];
        for i in 0..8 {
            bools[i] = ((value.inner >> i) % 2 == 1)
        }
        bools
    }
}

pub trait BitFlags: Sized + Clone + PartialEq + Default + ToBytes + FromBytes {
    const VALUES:[Option<Self> ; 8];
}

impl<BF:BitFlags> From<ArrayVec<BF, 8>> for BitField {
    fn from(value: ArrayVec<BF, 8>) -> Self {
        let mut bools = [false ; 8];
        for (i, val) in BF::VALUES.iter().enumerate() {
            match val {
                Some(flag) => if value.contains(flag) {
                    bools[i] = true;
                },
                None => ()
            }
            
        }

        bools.into()
    }
}

impl<BF:BitFlags> From<BitField> for ArrayVec<BF, 8> {
    fn from(value: BitField) -> Self {
        let bools = <BitField as Into<[bool ; 8]>>::into(value);
        let mut vec = ArrayVec::new(BF::default());
        for (i,bool) in bools.iter().enumerate() {
            if *bool {
                match &BF::VALUES[i] {
                    Some(flag) => vec.push(flag.clone()),
                    None => {
                        panic!("Trying to decode non-existent flag {}", i);
                    }
                }
            }
        }
        vec
    }
}