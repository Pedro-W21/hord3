#[derive(Clone, Copy, Debug)]
pub struct Trinary {
    first_trin:u8,
}

impl Trinary {
    pub fn new(trin:u8) -> Self {
        Self { first_trin:trin }
    }
    pub fn crz_op(&self, other:&Self) -> Trinary {
        match self.first_trin {
            0 => match other.first_trin {
                0 => Trinary::new(1),
                1 => Trinary::new(0),
                2 => Trinary::new(0),
                _ => panic!("OOOOOOOOOO")
            },
            1 => match other.first_trin {
                0 => Trinary::new(1),
                1 => Trinary::new(0),
                2 => Trinary::new(2),
                _ => panic!("IIIIIIIIIII")
            },
            2 => match other.first_trin {
                0 => Trinary::new(2),
                1 => Trinary::new(2),
                2 => Trinary::new(1),
                _ => panic!("DDDDDDDDDDd")
            }
            _ => panic!("AAAAAAAAAAAAAAAAA")
        }
    }
}