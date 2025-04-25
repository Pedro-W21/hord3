


pub struct MPMCConsumer<'a, IC:Iterator<Item = IT>, IT> {
    to_consume:&'a mut IC,

}

impl<'a, IC:Iterator<Item = IT>, IT> MPMCConsumer<'a, IC, IT> {
    // pub fn 
}