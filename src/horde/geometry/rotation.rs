use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use to_from_bytes_derive::{FromBytes, ToBytes};

use super::vec3d::Vec3Df;


/// Quaternion is a middle ground between Orientation and Rotation, it doesn't store all 3D rotation matrix coefficients as Rotation does, or angles as Orientation does
/// 
/// it is a representation of a Quaternion, which can be used to apply rotations to Vec3D structs directly. However, it computes the 3D rotation matrix for each rotation done this way, so it is recommended to use Orientation and Rotation instead for better performance as you rarely need to rotate only 1 Vec3D at a time
#[derive(Clone, Copy)]
pub struct Quaternion {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quaternion {
    fn new(w: f32, x: f32, y: f32, z: f32) -> Quaternion {
        Quaternion { w, x, y, z }
    }
    pub fn new_from_euler(yaw: f32, pitch: f32, roll: f32) -> Quaternion {
        let cy = (yaw * 0.5).cos();
        let cp = (pitch * 0.5).cos();
        let cr = (roll * 0.5).cos();

        let sy = (yaw * 0.5).sin();
        let sp = (pitch * 0.5).sin();
        let sr = (roll * 0.5).sin();

        let mut q = Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        q.w = cr * cp * cy + sr * sp * sy;
        q.x = sr * cp * cy - cr * sp * sy;
        q.y = cr * sp * cy + sr * cp * sy;
        q.z = cr * cp * sy - sr * sp * cy;

        q
    }
    #[inline(always)]
    pub fn rotate(&self, cible: Vec3Df) -> Vec3Df {
        Vec3Df::new(
            cible.x * (1.0 - 2.0*(self.y.powi(2) + self.z.powi(2))) + cible.y * (2.0 * (self.x * self.y - self.z * self.w)) + cible.z * (2.0 * (self.x * self.z + self.y * self.w)),
            cible.x * (2.0 * (self.x * self.y + self.z * self.w)) + cible.y * (1.0 - 2.0*(self.x.powi(2) + self.z.powi(2))) + cible.z * (2.0 * (self.y * self.z - self.x * self.w)),
            cible.x * (2.0 * (self.x * self.z - self.y * self.w)) + cible.y * (2.0 * (self.y * self.z + self.x * self.w)) + cible.z * (1.0 - 2.0*(self.x.powi(2) + self.y.powi(2)))
        )
        //let t = Vec3D::new(self.x, self.y, self.z).cross(&cible) * 2.0;
        //(cible + (t * self.w)) + Vec3D::new(self.x, self.y, self.z).cross(&t)
    }
    pub fn invert(&self) -> Self {
        let len = self.w.powi(2) + self.x.powi(2) + self.y.powi(2) + self.z.powi(2);
        Self { w: self.w/len, x: -self.x/len, y: -self.y/len, z: -self.z/len }
    }
    pub fn into_vec(&self) -> Vec3Df {
        self.rotate(Vec3Df::new(1.0, 0.0, 0.0))
    }
}


/// Rotation is a representation of a 3D rotation matrix computed using Euler angles
/// It is possible (and recommended) to use one Rotation to compute many transformations because Rotation contains the final computed forms of all coefficients in a 3D rotation matrix
/// however, a Rotation is thrice as memory-intensive as an Orientation, and doesn't store its rotation angles in an easily used way, so between phases of compute, it is generally better to store Orientation structs instead of Rotations if possible
#[derive(Debug, Clone, PartialEq, ToBytes, FromBytes)]
pub struct Rotation {
    pub p1:Vec3Df,
    pub p2:Vec3Df,
    pub p3:Vec3Df
}
impl Rotation {
    pub fn into_vec(&self) -> Vec3Df {
        self.rotate(Vec3Df::new(1.0, 0.0, 0.0))
    }
    pub fn from_orientation(orient:Orientation) -> Rotation {
        Self::new_from_euler(orient.yaw, orient.pitch, orient.roll)
    }
    pub fn new_from_euler(yaw: f32, pitch: f32, roll: f32) -> Rotation {
        let q1 = Quaternion::new_from_euler(yaw, pitch, roll);
        Rotation {
            p1:Vec3Df::new((1.0 - 2.0 * (q1.y.powi(2) + q1.z.powi(2))), (2.0 * (q1.x * q1.y - q1.z * q1.w)), (2.0 * (q1.x * q1.z + q1.y * q1.w))),
            p2:Vec3Df::new((2.0 * (q1.x * q1.y + q1.z * q1.w)), (1.0 - 2.0 * (q1.x.powi(2) + q1.z.powi(2))), (2.0 * (q1.y * q1.z - q1.x * q1.w))),
            p3:Vec3Df::new((2.0 * (q1.x * q1.z - q1.y * q1.w)), (2.0 * (q1.y * q1.z + q1.x * q1.w)), (1.0 - 2.0 * (q1.x.powi(2) + q1.y.powi(2))))
        }
    }
    pub fn new_from_quat(q1:Quaternion) -> Rotation {
        Rotation {
            p1:Vec3Df::new((1.0 - 2.0 * (q1.y.powi(2) + q1.z.powi(2))), (2.0 * (q1.x * q1.y - q1.z * q1.w)), (2.0 * (q1.x * q1.z + q1.y * q1.w))),
            p2:Vec3Df::new((2.0 * (q1.x * q1.y + q1.z * q1.w)), (1.0 - 2.0 * (q1.x.powi(2) + q1.z.powi(2))), (2.0 * (q1.y * q1.z - q1.x * q1.w))),
            p3:Vec3Df::new((2.0 * (q1.x * q1.z - q1.y * q1.w)), (2.0 * (q1.y * q1.z + q1.x * q1.w)), (1.0 - 2.0 * (q1.x.powi(2) + q1.y.powi(2))))
        }
    }
    pub fn new_from_inverted_orient(orient:Orientation) -> Rotation {
        Self::new_from_quat(Quaternion::new_from_euler(orient.yaw, orient.pitch, orient.roll).invert())
    }
    #[inline(always)]
    pub fn rotate(&self, cible: Vec3Df) -> Vec3Df {
        Vec3Df::new(
            cible.x * self.p1.x + cible.y * self.p1.y + cible.z * self.p1.z,
            cible.x * self.p2.x + cible.y * self.p2.y + cible.z * self.p2.z,
            cible.x * self.p3.x + cible.y * self.p3.y + cible.z * self.p3.z,
        )
    }
    pub fn rotate_mut(&self, cible: &mut Vec3Df) {
        *cible = Vec3Df::new(
            cible.x * self.p1.x + cible.y * self.p1.y + cible.z * self.p1.z,
            cible.x * self.p2.x + cible.y * self.p2.y + cible.z * self.p2.z,
            cible.x * self.p3.x + cible.y * self.p3.y + cible.z * self.p3.z,
        )
    }
    pub fn rotate_array_mut<const N:usize>(&self, array:&mut [Vec3Df ; N]) {
        for pos in array {
            *pos = self.rotate(*pos);
        }
    }
    pub fn rotate_array<const N:usize>(&self, array:&[Vec3Df ; N]) -> [Vec3Df ; N] {
        let mut cloned = array.clone();
        self.rotate_array_mut(&mut cloned);
        cloned
    }
}
/// Orientation is the standard type used to describe a 3D object's orientation compared to another set of axis
/// To compute the rotation of one or many Vec3D structs based on an orientation, use Rotation::from_orientation and use the resulting Rotation for computing the transformations
/// 
/// As a reminder, when X is forward, Y is sideways and Z is up :
/// - yaw is rotation around the Z axis
/// - pitch is rotation around the Y axis
/// - roll is rotation around the X axis
#[derive(Clone, Copy, Debug, PartialEq, ToBytes, FromBytes)]
pub struct Orientation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl Orientation {
    pub fn zero() -> Orientation {
        Orientation {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        }
    }
    pub fn new(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self { yaw, pitch, roll }
    }
    pub fn into_vec(&self) -> Vec3Df {
        Vec3Df::new_orient((self.yaw, self.pitch))
    }
    pub fn from_to(p1: Vec3Df, p2: Vec3Df) -> Self {
        let (yaw, pitch) = p1.get_orient_vers(&p2);
        Self::new(yaw, pitch, 0.0)
    }
}

impl Add<Self> for Orientation {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Orientation::new(self.yaw + rhs.yaw, self.pitch + rhs.pitch, self.roll + rhs.roll)
    }
}

impl Add<Self> for &Orientation {
    type Output = Orientation;
    fn add(self, rhs: Self) -> Self::Output {
        Orientation::new(self.yaw + rhs.yaw, self.pitch + rhs.pitch, self.roll + rhs.roll)
    }
}

impl Sub<Self> for &Orientation {
    type Output = Orientation;
    fn sub(self, rhs: Self) -> Self::Output {
        Orientation::new(self.yaw - rhs.yaw, self.pitch - rhs.pitch, self.roll - rhs.roll)
    }
}

impl Sub<Self> for Orientation {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Orientation::new(self.yaw - rhs.yaw, self.pitch - rhs.pitch, self.roll - rhs.roll)
    }
}

impl AddAssign<Self> for Orientation {
    fn add_assign(&mut self, rhs: Self) {
        self.yaw += rhs.yaw; 
        self.pitch += rhs.pitch;
        self.roll += rhs.roll;
    }
}

impl SubAssign<Self> for Orientation {
    fn sub_assign(&mut self, rhs: Self) {
        self.yaw -= rhs.yaw; 
        self.pitch -= rhs.pitch;
        self.roll -= rhs.roll;
    }
}

impl Mul<f32> for Orientation {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Orientation::new(self.yaw * rhs, self.pitch * rhs, self.roll * rhs)
    }
}
