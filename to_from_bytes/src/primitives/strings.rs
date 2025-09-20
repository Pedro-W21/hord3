use crate::{ToBytes, ByteDecoder, FromBytes};

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
    got_len:bool,
    counter:usize,
    utf_8_counter:u8,
    end_string:String,
}

impl FromBytes for String {
    type Decoder = StringDecoder;
    fn get_decoder() -> Self::Decoder {
        StringDecoder {
            got_len:false,
            counter:8,
            end_string:String::new(),
            utf_8_counter:0,
        }
    }
}

impl ByteDecoder<String> for StringDecoder {
    fn decode_byte(&mut self,bytes:&mut Vec<u8>, byte:u8) -> Option<String> {
        if self.got_len {
            self.counter -= 1;
            bytes.push(byte);
            if self.counter == 0 {
                let string_out = String::from_utf8(bytes.clone()).unwrap();
                bytes.clear();
                Some(string_out)
            }
            else {
                None
            }
        }
        else {
            self.counter -= 1;
            bytes.push(byte);
            if self.counter == 0 {
                self.counter = usize::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3],bytes[4], bytes[5], bytes[6], bytes[7]]);
                self.got_len = true;
                bytes.clear();
            }
            if self.counter == 0 {
                bytes.clear();
                Some(String::new())
            }
            else {
                None
            }
            
        }
    }
}