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

/*
/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::collections::HashSet;
use std::io::{self, BufRead};

struct Graphe {
    envergure:f32,
    noeuds:Vec<Noeud>,
}

impl Graphe {
    fn add_voisins_to(&mut self, id:usize) {
        let start = self.noeuds[id].clone();
        for i in 0..id {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
        for i in (id+1)..self.noeuds.len() {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
    }
    fn add_voisins_to_all(&mut self) {
        for i in 0..self.noeuds.len() {
            self.add_voisins_to(i);
        }
    }
    fn paths_with_least_nodes_between(&self, start:usize, end:usize) -> Vec<(Vec<usize>, HashSet<usize>)> {
        let mut paths = vec![(vec![start], HashSet::from(vec![start]))];
        let mut ended = false;
        while !ended {
            let mut new_paths = Vec::with_capacity(paths.capacity());
            let mut any_progress = false;
            for (path, explored) in &paths {
                let mut last = path.last().unwrap();
                for neighbor in self.noeuds[last] {
                    let mut new_explored = explored.clone();
                    let mut new_path = path.clone();
                    if !explored.contains(&neighbor) {
                        new_explored.insert(neighbor);
                        new_path.push(neighbor);
                        if neighbor == end {
                            ended = true;
                        }
                        any_progress = true;
                        new_paths.push((new_path, new_explored));
                    }
                }
            }
            if !any_progress {
                ended = true;
            }
        }
    }
}

#[derive(Clone)]
struct Noeud {
    posx:f32,
    posy:f32,
    voisins:Vec<usize>
}

impl Noeud {
    fn dist(&self, other:&Self) -> f32 {
        ((self.posx - other.posx).powi(2) + (self.posy - other.posy).powi(2)).sqrt()
    }
}

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let mut lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let envergure = i32::from_str(lines[0].trim()).unwrap() as f32;
    
    let mut graphe = Graphe {
        envergure,
        noeuds:Vec::with_capacity(600),
    };
    
    lines.remove(3);
    
    for (i, line) in lines.into_iter().skip(1).enumerate() {
        let coords:Vec<&str> = line.split_whitespace().collect();
        let posx = i32::from_str(coords[0]).unwrap() as f32;
        let posy = i32::from_str(coords[1]).unwrap() as f32;
        graphe.noeuds.push(Noeud {posx, posy, voisins:Vec::with_capacity(10)});
    }
    
    graphe.add_voisins_to_all();
    
    // You can now process the lines as needed
    //println!("{}", name);

    // Example of using eprintln for debugging information
    // eprintln!("Debugging information: {:?} {:?}", finishers, winning_time);
}
*/

// notes isograd
/*
1er exo :
simple, efficace, parfait



code pour le 2 :

/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::io::{self, BufRead};

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let mut first_total = ("nope".to_string(),-1, -1, -1);
    for line in lines.into_iter().skip(1) {
        let splitted:Vec<&str> = line.split_whitespace().collect();
        let name = splitted[0];
        let gold = i32::from_str(splitted[1]).unwrap();
        let silver = i32::from_str(splitted[2]).unwrap();
        let bronze = i32::from_str(splitted[3]).unwrap();
        
        if gold >= first_total.1 && silver >= first_total.2 && bronze >= first_total.3 {
            first_total.0 = name.to_string();
        }
    }

    // You can now process the lines as needed
    println!("{}", name);

    // Example of using eprintln for debugging information
    //eprintln!("Debugging information: {:?}", lines);
}

notes 2ème exo:
    - pas vraiment + compliqué que le premier, même ptet + simple (le split est + direct, y'a pas d'égalité de string)
  - le classement gold puis silver puis bronze peut être fourbe mais c'est un bon challenge

3ème exo code :

/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::collections::HashSet;
use std::io::{self, BufRead};

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let first_line:Vec<&str> = lines[0].split_whitespace().collect();
    let height = usize::from_str(first_line[0]).unwrap();
    let width = usize::from_str(first_line[1]).unwrap();
    
    let mut directions:Vec<i32> = vec![0 ; height * width];
    
    for (i, line) in lines.into_iter().skip(1).enumerate() {
        for (j, dir) in line.chars().enumerate() {
            directions[j + i * width] = match dir {
                '>' => 1,
                '<' => -1,
                '^' => -(width as i32),
                'v' => (width as i32),
                _ => panic!("C'est vraiment bizarre ça dis donc")
            };
        }
    }
    
    let mut target = height * width;
    
    let mut finishers:Vec<(usize, usize)> = Vec::with_capacity(width);
    
    let mut winning_time = usize::MAX;
    
    for start in 0..width {
        let mut explored = HashSet::with_capacity(height * 2);
        let mut pos:i32 = start as i32;
        let mut secs = 0;
        while (pos as usize) < target && !explored.contains(&pos) {
            explored.insert(pos);
            pos += directions[pos as usize];
            secs += 1;
        }
        if pos as usize >= target {
            finishers.push((start, secs));
            if secs < winning_time {
                winning_time = secs;
            }
        }
    }
    
    for (i, time) in finishers.clone() {
        if time == winning_time {
            print!("{} ", i + 1);
        }
    }

    // You can now process the lines as needed
    //println!("{}", name);

    // Example of using eprintln for debugging information
    eprintln!("Debugging information: {:?} {:?}", finishers, winning_time);
}

    notes exo 3 : 
    - gros step de difficulté
    - les boucles possibles vont être un gros piège (contournable lorsque l'on comprends l'énoncé mais faut y penser)
    - je sais pas si l'output prend en compte les gagnants dans un ordre pas croissant
    - la résolution naÏve (tout modéliser en 2D avec des vraies position) est probablement longue
    - peu de connaissances en algo demandées, et le manque de collisions est très bien pensé

*/


/*
code pour le 4 : 

/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::collections::HashSet;
use std::io::{self, BufRead};

struct Graphe {
    envergure:f32,
    noeuds:Vec<Noeud>,
}

impl Graphe {
    fn add_voisins_to(&mut self, id:usize) {
        let start = self.noeuds[id].clone();
        for i in (8.max(id) - 8)..id {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
        for i in (id+1)..(id + 8).min(self.noeuds.len()) {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
    }
    fn add_voisins_to_all(&mut self) {
        for i in 0..self.noeuds.len() {
            self.add_voisins_to(i);
        }
    }
    fn paths_with_least_nodes_between(&self, start:usize, end:usize) -> Vec<Vec<usize>> {
        let mut start_hashset = HashSet::new();
        start_hashset.insert(start);
        let mut paths = vec![(vec![start], start_hashset)];
        let mut ended = false;
        while !ended {
            let mut new_paths = Vec::with_capacity(paths.capacity());
            let mut any_progress = false;
            for (path, explored) in &paths {
                let mut last = path.last().unwrap();
                for neighbor in &self.noeuds[*last].voisins {
                    let mut new_explored = explored.clone();
                    let mut new_path = path.clone();
                    if !explored.contains(neighbor) {
                        new_explored.insert(*neighbor);
                        new_path.push(*neighbor);
                        if *neighbor == end {
                            ended = true;
                        }
                        any_progress = true;
                        new_paths.push((new_path, new_explored));
                    }
                }
            }
            if !any_progress {
                ended = true;
            }
        }
        let mut minimum = usize::MAX;
        let mut minimum_paths = Vec::with_capacity(paths.len());
        for (path, explored) in &paths {
            if path.len() < minimum && explored.contains(&end) {
                minimum = path.len();
            }
        }
        for (path, explored) in paths {
            if path.len() == minimum && explored.contains(&end) {
                minimum_paths.push(path);
            }
        }
        minimum_paths
    }
}

#[derive(Clone)]
struct Noeud {
    posx:f32,
    posy:f32,
    voisins:Vec<usize>
}

impl Noeud {
    fn dist(&self, other:&Self) -> f32 {
        ((self.posx - other.posx).powi(2) + (self.posy - other.posy).powi(2)).sqrt()
    }
}

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let mut lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let envergure = i32::from_str(lines[0].trim()).unwrap() as f32;
    
    let mut graphe = Graphe {
        envergure,
        noeuds:Vec::with_capacity(600),
    };
    
    lines.remove(3);
    
    for (i, line) in lines.into_iter().skip(1).enumerate() {
        let coords:Vec<&str> = line.split_whitespace().collect();
        let posx = i32::from_str(coords[0]).unwrap() as f32;
        let posy = i32::from_str(coords[1]).unwrap() as f32;
        graphe.noeuds.push(Noeud {posx, posy, voisins:Vec::with_capacity(10)});
    }
    
    graphe.add_voisins_to_all();
    
    let mut possible = graphe.paths_with_least_nodes_between(0, 1);
    if possible.len() > 0 {
        for step in possible[0].clone() {
            println!("{} {}", graphe.noeuds[step].posx as i32, graphe.noeuds[step].posy as i32);
        }
    }
    else {
        println!("-1");
    }
    
    // You can now process the lines as needed
    //println!("{}", name);

    // Example of using eprintln for debugging information
    // eprintln!("Debugging information: {:?} {:?}", finishers, winning_time);
}

*/

/*
autre code 4 

/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::collections::HashSet;
use std::io::{self, BufRead};

struct Graphe {
    envergure:f32,
    noeuds:Vec<Noeud>,
}

impl Graphe {
    fn add_voisins_to(&mut self, id:usize) {
        let start = self.noeuds[id].clone();
        for i in (8.max(id) - 8)..id {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
        for i in (id+1)..(id + 8).min(self.noeuds.len()) {
            if start.dist(&self.noeuds[i]) <= self.envergure {
                self.noeuds[id].voisins.push(i);
            }
        }
    }
    fn add_voisins_to_all(&mut self) {
        for i in 0..self.noeuds.len() {
            self.add_voisins_to(i);
        }
    }
    fn paths_with_least_nodes_between(&self, start:usize, end:usize) -> Vec<Vec<usize>> {
        let mut start_hashset = HashSet::new();
        start_hashset.insert(start);
        let mut paths = vec![(vec![start], start_hashset)];
        let mut ended = false;
        while !ended {
            let mut new_paths = Vec::with_capacity(paths.capacity());
            let mut any_progress = false;
            for (path, explored) in &paths {
                let mut last = path.last().unwrap();
                for neighbor in &self.noeuds[*last].voisins {
                    let mut new_explored = explored.clone();
                    let mut new_path = path.clone();
                    if !explored.contains(neighbor) {
                        new_explored.insert(*neighbor);
                        new_path.push(*neighbor);
                        if *neighbor == end {
                            ended = true;
                        }
                        any_progress = true;
                        
                    }
                    new_paths.push((new_path, new_explored));
                }
            }
            paths = new_paths;
            if !any_progress {
                ended = true;
            }
        }
        eprintln!("------------------------------------\n FINISHED COMPUTE OF 1 \n ------------------------");
        let mut minimum = usize::MAX;
        let mut minimum_paths = Vec::with_capacity(paths.len());
        for (path, explored) in &paths {
            if path.len() < minimum && explored.contains(&end) {
                minimum = path.len();
            }
        }
        for (path, explored) in paths {
            if path.len() == minimum && explored.contains(&end) {
                minimum_paths.push(path);
            }
        }
        minimum_paths
    }
}

#[derive(Clone)]
struct Noeud {
    posx:f32,
    posy:f32,
    voisins:Vec<usize>
}

impl Noeud {
    fn dist(&self, other:&Self) -> f32 {
        ((self.posx - other.posx).powi(2) + (self.posy - other.posy).powi(2)).sqrt()
    }
}

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let mut lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let envergure = i32::from_str(lines[0].trim()).unwrap() as f32;
    
    let mut graphe = Graphe {
        envergure,
        noeuds:Vec::with_capacity(600),
    };
    
    lines.remove(3);
    
    for (i, line) in lines.into_iter().skip(1).enumerate() {
        let coords:Vec<&str> = line.split_whitespace().collect();
        let posx = i32::from_str(coords[0]).unwrap() as f32;
        let posy = i32::from_str(coords[1]).unwrap() as f32;
        graphe.noeuds.push(Noeud {posx, posy, voisins:Vec::with_capacity(10)});
    }
    
    graphe.add_voisins_to_all();
    
    let mut possible = graphe.paths_with_least_nodes_between(0, 1);
    if possible.len() > 0 {
        for step in possible[0].clone() {
            println!("{} {}", graphe.noeuds[step].posx as i32, graphe.noeuds[step].posy as i32);
        }
    }
    else {
        println!("-1");
    }
    
    // You can now process the lines as needed
    //println!("{}", name);

    // Example of using eprintln for debugging information
    // eprintln!("Debugging information: {:?} {:?}", finishers, winning_time);
}

*/

/*
TEST NUL

let mut start_hashset = HashSet::new();
        start_hashset.insert(start);
        let mut paths = vec![(vec![start], start_hashset)];
        let mut ended = false;
        let mut valid_paths = Vec::new();
        while !ended  {
            let mut path = vec![start];
            let mut tried_at = vec![HashSet::new()];
            let mut all_explored = HashSet::new();
            all_explored.insert(start);
            path.len() > 0 {
                let mut tries = tried_at.last_mut().unwrap();
                let latest_point = path.last().unwrap();
                if self.noeuds[latest_point].voisins.len() > 0 {
                    let to_try = self.noeuds[latest_point].voisins.pop().unwrap();
                    if !all_explored.contains(&to_try) {
                        all_explored.insert(to_try);
                        path.push(to_try);
                    }
                    tries.insert(to_try);
                }
            }
        }
        
        
*/

/*
exo 4 notes:
- trop compliqué comme défi d'optimisation, même avec la bonne idée il faut bien la coder pour que ça fonctionne
- le problème est pas assez restreint pour avoir une solution "intelligente" qui marche pour sur, on est obligés de sortir les grands moyens (à priori)

*/

/*
/*******
* Read input from System.in
* Use: println to output your result to STDOUT.
* Use: eprintln to output debugging information to STDERR.
* ***/
use std::str::FromStr;
use std::collections::HashSet;
use std::io::{self, BufRead};

#[derive(Clone, Debug)]
struct SacTest {
    items:Vec<(i32, i32)>,
    current_score:i32,
    current_time:i32,
}

#[derive(Clone, Debug)]
struct Sac {
    tries:Vec<SacTest>,
    current_best_score:i32,
}

impl Sac {
    fn do_next_iteration(&mut self) -> bool {
        let mut new_tests = Vec::with_capacity(self.tries.len());
        let mut still_moving = false;
        for test in &self.tries {
            if test.items.len() > 0 {
                still_moving = true;
                // eprintln!("{}", self.tries.len());
            }
            for (i, (points, time)) in test.items.iter().enumerate() {
                let mut cloned = test.clone();
                if cloned.current_time >= *time {
                    cloned.current_score += *points;
                    cloned.current_time -= *time;
                    if *points == 1 {
                        cloned.items.remove(i);
                    }
                    else {
                        cloned.items[i].0 -= 1;
                    }
                    if cloned.current_score >= self.current_best_score {
                        self.current_best_score = cloned.current_score;
                    }
                    new_tests.push(cloned);
                }
            }
        }
        self.tries = new_tests;
        still_moving
    }
}

fn main() {
    // Read input from System.in
    let stdin = io::stdin();
    let mut lines: Vec<String> = stdin.lock().lines()
      .map(|line| line.expect("Could not read line"))
      .collect();
      
    let mut secondes = i32::from_str(lines[0].trim()).unwrap();
    let figures = i32::from_str(lines[1].trim()).unwrap();
    
    let mut points = 0;
    
    let mut data:Vec<(i32, i32)> = Vec::with_capacity(figures as usize);
    
    let mut sac = Sac {
        tries:Vec::with_capacity(10),
        current_best_score:0,
    };
    
    for (i, line) in lines.into_iter().skip(2).enumerate() {
        let values:Vec<&str> = line.split_whitespace().collect();
        let temps = i32::from_str(values[0]).unwrap();
        let points = i32::from_str(values[1]).unwrap();
        data.push((points, temps));
    }
    sac.tries.push(SacTest {items:data, current_score:0, current_time:secondes});
    while sac.do_next_iteration() {
        
    }
    
    println!("{}", sac.current_best_score);
    //eprintln!("{:?}", sac.penu2_tries);
    eprintln!("---------------------")
    // You can now process the lines as needed
    //println!("{}", name);

    // Example of using eprintln for debugging information
    
}

*/