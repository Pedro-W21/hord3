use std::{hash::Hash, ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign}};

use to_from_bytes::{FromBytes, ToBytes};
use to_from_bytes_derive::{FromBytes, ToBytes};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Coord {
    X,
    Y,
    Z
}

impl Coord {
    pub fn get_others(self) -> [Coord ; 2] {
        match self {
            Self::X => [Self::Y, Self::Z],
            Self::Y => [Self::X, Self::Z],
            Self::Z => [Self::X, Self::Y],
        }
    }
    pub fn get_last(self, other:Self) -> Coord {
        if self == Self::X && other == Self::Y || self == Self::Y && other == Self::X {
            Self::Z
        }
        else if self == Self::Z && other == Self::Y || self == Self::Y && other == Self::Z {
            Self::X
        }
        else {
            Self::Y
        }
    }
}

pub trait Number: ToBytes + FromBytes + PartialOrd + PartialEq + Add<Self, Output = Self> + Sub<Self, Output = Self> + Div<Self, Output = Self> + Mul<Self, Output = Self> + Neg<Output = Self> + AddAssign<Self> + SubAssign<Self> + MulAssign<Self> + DivAssign<Self> + Sized + Clone + Copy {
    const ONE:Self;
    const ZERO:Self;
    fn get_analog_for_hash(&self) -> u64;
}

impl Number for f32 {
    const ONE:Self = 1.0;
    const ZERO:Self = 0.0;
    fn get_analog_for_hash(&self) -> u64 {
        (*self as f64).to_bits()
    }
}

impl Number for f64 {
    const ONE:Self = 1.0;
    const ZERO:Self = 0.0;
    fn get_analog_for_hash(&self) -> u64 {
        self.to_bits()
    }
}

impl Number for i32 {
    const ONE:Self = 1;
    const ZERO:Self = 0;
    fn get_analog_for_hash(&self) -> u64 {
        unsafe {std::mem::transmute(*self as i64)}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, ToBytes, FromBytes)]
pub struct Vec3D<N:Number> {
    pub x: N,
    pub y: N,
    pub z: N,
    //filler:f32
}

impl<N:Number> Hash for Vec3D<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.get_analog_for_hash().hash(state);

    }
}

impl<N:Number> Eq for Vec3D<N> {

}

impl<N:Number> Vec3D<N> {
    pub const fn new(x: N, y: N, z: N) -> Vec3D<N> {
        Vec3D { x, y, z }
    }
    pub const fn all_ones() -> Vec3D<N> {
        Vec3D { x: N::ONE, y: N::ONE, z: N::ONE }
    }
    
    pub fn get_cube_vertices_around(&self, scale:N) -> [Vec3D<N> ; 8] {
        let x = self.x;
        let y = self.y;
        let z = self.z;
        [
            Vec3D::new(x - scale, y - scale, z - scale),//0
            Vec3D::new(x + scale, y - scale, z - scale),//1
            Vec3D::new(x - scale, y - scale, z + scale),//2
            Vec3D::new(x + scale, y - scale, z + scale),//3
            Vec3D::new(x - scale, y + scale, z - scale),//4
            Vec3D::new(x + scale, y + scale, z - scale),//5
            Vec3D::new(x - scale, y + scale, z + scale),//6
            Vec3D::new(x + scale, y + scale, z + scale) //7

        ]
    }
    pub fn coords_to_array(&self) -> [N ; 3] {
        [self.x, self.y, self.z]
    }
    pub fn in_origin_prism(&self, length:N, width:N, height:N) -> bool {
        self.x >= N::ZERO && self.x < length && self.y >= N::ZERO && self.y < width && self.z >= N::ZERO && self.z < height
    }
    
    pub fn clamp(&self, minx:N, miny:N, minz:N, maxx:N, maxy:N, maxz:N) -> Vec3D<N> {
        Vec3D::new(
            if self.x < minx {minx} else if self.x > maxx {maxx} else {self.x},
            if self.y < miny {miny} else if self.y > maxy {maxy} else {self.y},
            if self.z < minz {minz} else if self.z > maxz {maxz} else {self.z}
        )
    }
    pub fn positive(&self) -> bool {
        self.x > N::ZERO && self.y > N::ZERO && self.z > N::ZERO
    }
    
    pub fn zero() -> Vec3D<N> {
        Vec3D::new(N::ZERO, N::ZERO, N::ZERO)
    }
    
    pub fn dot(&self, autre: &Self) -> N {
        self.x * autre.x + self.y * autre.y + self.z * autre.z
    }
    
    pub const fn co(&self, coord:Coord) -> N {
        match coord {
            Coord::X => self.x,
            Coord::Y => self.y,
            Coord::Z => self.z
        }
    }
    
    
    #[inline(always)]
    pub fn cross(&self, autre: &Self) -> Vec3D<N> {
        Vec3D::new(
            self.y * autre.z - self.z * autre.y,
            self.z * autre.x - self.x * autre.z,
            self.x * autre.y - self.y * autre.x,
        )
    }
    pub fn sum_components(&self) -> N {
        self.x + self.y + self.z
    }
    pub fn component_product(&self, other:&Vec3D<N>) -> Vec3D<N> {
        Vec3D { x: self.x * other.x, y: self.y * other.y, z: self.z * other.z }
    }
    pub fn component_div(&self, other:&Vec3D<N>) -> Vec3D<N> {
        Vec3D { x: self.x / other.x, y: self.y / other.y, z: self.z / other.z }
    }
    pub fn mut_component_product(&mut self, other:&Vec3D<N>) {
        self.x *= other.x;
        self.y *= other.y;
        self.z *= other.z;
    }
}

impl<N:Number> Neg for Vec3D<N> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Vec3D::new(-self.x, -self.y, -self.z)
    }
}
impl<N:Number> Add<Self> for Vec3D<N> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3D::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<N:Number> AddAssign<Self> for Vec3D<N> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl<N:Number> Sub<Self> for Vec3D<N> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3D::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl<N:Number> SubAssign<Self> for Vec3D<N> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl<N:Number> Mul<N> for Vec3D<N> {
    type Output = Self;
    fn mul(self, rhs: N) -> Self::Output {
        Vec3D::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl<N:Number> MulAssign<N> for Vec3D<N> {
    fn mul_assign(&mut self, rhs: N) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl<N:Number> Div<N> for Vec3D<N> {
    type Output = Self;
    fn div(self, rhs: N) -> Self::Output {
        Vec3D::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl<N:Number> DivAssign<N> for Vec3D<N> {
    fn div_assign(&mut self, rhs: N) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

impl<N:Number> Neg for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn neg(self) -> Self::Output {
        Vec3D::new(-self.x, -self.y, -self.z)
    }
}
impl<N:Number> Add<&Self> for Vec3D<N> {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self::Output {
        Vec3D::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<N:Number> AddAssign<&Self> for Vec3D<N> {
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl<N:Number> Sub<&Self> for Vec3D<N> {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self::Output {
        Vec3D::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}


impl<N:Number> Add<Vec3D<N>> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn add(self, rhs: Vec3D<N>) -> Self::Output {
        Vec3D::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<N:Number, const K:usize> Add<Vec3D<N>> for [Vec3D<N> ; K] {
    type Output = Self;
    fn add(self, rhs: Vec3D<N>) -> Self::Output {
        self.map(|vec| {vec + rhs})
    }
} 

impl<N:Number> Sub<Vec3D<N>> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn sub(self, rhs: Vec3D<N>) -> Self::Output {
        Vec3D::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl<N:Number> Add<Self> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3D::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<N:Number> Sub<Self> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3D::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl<N:Number> SubAssign<&Self> for Vec3D<N> {
    fn sub_assign(&mut self, rhs: &Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl<N:Number> Mul<N> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn mul(self, rhs: N) -> Self::Output {
        Vec3D::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl<N:Number> Div<N> for &Vec3D<N> {
    type Output = Vec3D<N>;
    fn div(self, rhs: N) -> Self::Output {
        Vec3D::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

pub type Vec3Df = Vec3D<f32>;

impl Vec3Df {
    pub fn to_i32_if_in_prism(&self, prism_start:Vec3Df, prism_end:Vec3Df) -> Option<Vec3D<i32>> {
        if self.x >= prism_start.x && self.y >= prism_start.y && self.z >= prism_start.z && self.x < prism_end.x && self.y < prism_end.y && self.z < prism_end.z {
            Some(Vec3D::new(self.x as i32, self.y as i32, self.z as i32))
        }
        else {
            None 
        }
    }
    pub fn to_i32_prism_clamped(&self, prism_start:Vec3Df, prism_end:Vec3Df) -> Vec3D<i32> {
        let x = if self.x < prism_start.x {
            prism_start.x as i32
        }
        else if self.x >= prism_end.x {
            prism_end.x as i32 - 1
        }
        else {
            self.x as i32
        };
        let y = if self.y < prism_start.y {
            prism_start.y as i32
        }
        else if self.y >= prism_end.y {
            prism_end.y as i32 - 1
        }
        else {
            self.y as i32
        };
        let z = if self.z < prism_start.z {
            prism_start.z as i32
        }
        else if self.z >= prism_end.z {
            prism_end.z as i32 - 1
        }
        else {
            self.z as i32
        };
        Vec3D::new(x, y, z)
    }
    pub fn to_usize_if_in_orig_prism(&self, length:f32, width:f32, height:f32) -> Option<(usize,usize,usize)> {
        if self.in_origin_prism(length, width, height) {
            //dbg!(self);
            Some((self.x as usize, self.y as usize, self.z as usize))
        }
        else {
            None
        }
    }
    pub fn to_u_orig_prism_clamped(&self, length:f32, width:f32, height:f32) -> (usize,usize,usize) {
        let x = if self.x < 0.0 {
            0
        }
        else if self.x >= length {
            length as usize - 1
        }
        else {
            self.x as usize
        };
        let y = if self.y < 0.0 {
            0
        }
        else if self.y >= width {
            width as usize - 1
        }
        else {
            self.y as usize
        };
        let z = if self.z < 0.0 {
            0
        }
        else if self.z >= height {
            height as usize - 1
        }
        else {
            self.z as usize
        };
        (x,y,z)

    }
    pub fn angle_entre(&self, autre: &Self) -> f32 {
        (self.dot(autre) / (self.norme() * autre.norme())).acos()
    }
    pub fn new_orient((angh, angv): (f32, f32)) -> Vec3Df {
        Vec3D::new(angh.cos() * angv.sin(), angh.sin() * angv.sin(), angv.cos())
    }
    pub fn get_orient_vers(&self, cible: &Self) -> (f32, f32) {
        let dist_horiz = ((cible.x - self.x).powi(2) + (cible.y - self.y).powi(2)).sqrt();
        (
            (cible.y - self.y).atan2(cible.x - self.x),
            (dist_horiz).atan2(cible.z - self.z),
        )
    }
    pub fn get_orient_from_forward(&self) -> (f32, f32) {
        Vec3D::new(1.0, 0.0, 0.0).get_orient_vers(self)
    }
    pub fn new_orient_vers(&self, autre: &Self) -> Vec3Df {
        Vec3D::new_orient(self.get_orient_vers(autre))
    }
    pub fn dist(&self, autre: &Self) -> f32 {
        ((autre.x - self.x).powi(2) + (autre.y - self.y).powi(2) + (autre.z - self.z).powi(2))
            .sqrt()
    }
    pub fn det2D(&self, other:&Vec3Df) -> f32 {
        self.x * other.y - self.y * other.x
    }
    pub fn dist_2D_x_z(&self, autre: &Self) -> f32 {
        ((autre.x - self.x).powi(2) + (autre.z - self.z).powi(2)).sqrt()
    }
    pub fn dist_squared(&self, autre: &Self) -> f32 {
        (autre.x - self.x).powi(2) + (autre.x - self.x).powi(2) + (autre.x - self.x).powi(2)
    }
    pub fn roughly_under(&self, threshold:f32) -> bool {
        self.x.abs() < threshold && self.y.abs() < threshold && self.z.abs() < threshold
    }
    pub fn div_floor(&self, rhs:Vec3Df) -> Self {
        let divved = Vec3D::new(self.x/rhs.x, self.y/rhs.y, self.z/rhs.z);
        Self { x: if divved.x.is_sign_negative() {(divved.x - 1.0).trunc()} else {divved.x.trunc()}, y: if divved.y.is_sign_negative() {(divved.y - 1.0).trunc()} else {divved.y.trunc()}, z: if divved.z.is_sign_negative() {(divved.z - 1.0).trunc()} else {divved.z.trunc()} }
    }
    pub fn mul_floor(&self, rhs:f32) -> Self {
        let mulled = self * rhs;
        Self { x: if mulled.x.is_sign_negative() {(mulled.x - 1.0).trunc()} else {mulled.x.trunc()}, y: if mulled.y.is_sign_negative() {(mulled.y - 1.0).trunc()} else {mulled.y.trunc()}, z: if mulled.z.is_sign_negative() {(mulled.z - 1.0).trunc()} else {mulled.z.trunc()} }
    
    }
    pub fn norme(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2)).sqrt()
    }
    pub fn norme_square(&self) -> f32 {
        self.x.powi(2) + self.y.powi(2) + self.z.powi(2)
    }
    pub fn normalise(&self) -> Vec3Df {
        let long = self.norme();
        Vec3D::new(self.x / long, self.y / long, self.z / long)
    }
    /// Safety : the resulting f32 value is not supposed to be read as-is, and is only carrying the packed data for future unpacking
    pub unsafe fn pack_f32(&self) -> f32 {
        unsafe {f32::from_le_bytes([
         std::mem::transmute::<i8, u8>(self.x as i8),
         std::mem::transmute::<i8, u8>(self.y as i8),
         std::mem::transmute::<i8, u8>(self.z as i8),
         0
         ])}
    }
    /// Safety : the resulting u32 value is not supposed to be read as-is, and is only carrying the packed data for future unpacking
    pub unsafe fn pack_u32(&self) -> u32 {
        unsafe {u32::from_le_bytes([
            std::mem::transmute::<i8, u8>(self.x.to_int_unchecked()),
            std::mem::transmute::<i8, u8>(self.y.to_int_unchecked()),
            std::mem::transmute::<i8, u8>(self.z.to_int_unchecked()),
            0
            ])}
    }
    pub fn normalize_127_pack(&self) -> u32 {
        let length = 1.0/self.norme();
        unsafe {
            (self * length * 127.0).pack_u32()
        }
    }
}