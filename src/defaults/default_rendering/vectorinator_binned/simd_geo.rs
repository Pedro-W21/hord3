use std::{ops::{Add, Sub}, simd::Simd};

use crate::horde::geometry::{rotation::Rotation, vec3d::{Vec3D, Vec3Df}, HordeFloat};

pub const LANE_COUNT:usize = 4;
pub const LANE_COUNT_I32:i32 = LANE_COUNT as i32;
pub const LANE_COUNT_U32:u32 = LANE_COUNT as u32;
pub const LANE_COUNT_F32:f32 = LANE_COUNT as f32;
pub const INV_LANE_COUNT_F32:f32 = 1.0/LANE_COUNT_F32;


pub struct SIMDRotation {
    p1x:Simd<HordeFloat, LANE_COUNT>,
    p1y:Simd<HordeFloat, LANE_COUNT>,
    p1z:Simd<HordeFloat, LANE_COUNT>,
    p2x:Simd<HordeFloat, LANE_COUNT>,
    p2y:Simd<HordeFloat, LANE_COUNT>,
    p2z:Simd<HordeFloat, LANE_COUNT>,
    p3x:Simd<HordeFloat, LANE_COUNT>,
    p3y:Simd<HordeFloat, LANE_COUNT>,
    p3z:Simd<HordeFloat, LANE_COUNT>,
}

impl SIMDRotation {
    pub fn from_rotation(rotat:&Rotation) -> Self {
        Self {
            p1x: Simd::splat(rotat.p1.x),
            p1y: Simd::splat(rotat.p1.y),
            p1z: Simd::splat(rotat.p1.z),
            p2x: Simd::splat(rotat.p2.x),
            p2y: Simd::splat(rotat.p2.y),
            p2z: Simd::splat(rotat.p2.z),
            p3x: Simd::splat(rotat.p3.x),
            p3y: Simd::splat(rotat.p3.y),
            p3z: Simd::splat(rotat.p3.z)
        }
    }
    pub fn rotate(&self, vector:SIMDVec3Df) -> SIMDVec3Df {
        SIMDVec3Df::new(
            vector.x * self.p1x + vector.y * self.p1y + vector.z * self.p1z,
            vector.x * self.p2x + vector.y * self.p2y + vector.z * self.p2z,
            vector.x * self.p3x + vector.y * self.p3y + vector.z * self.p3z,
        )
    }
}

#[derive(Clone)]

pub struct SIMDVec3Df {
    pub x:Simd<HordeFloat, LANE_COUNT>,
    pub y:Simd<HordeFloat, LANE_COUNT>,
    pub z:Simd<HordeFloat, LANE_COUNT>,
}

impl SIMDVec3Df {
    pub fn new(x:Simd<HordeFloat, LANE_COUNT>, y:Simd<HordeFloat, LANE_COUNT>, z:Simd<HordeFloat, LANE_COUNT>) -> Self {
        Self { x, y, z }
    }
    pub fn from_vec3D(vec:&Vec3Df) -> Self {
        Self { x: Simd::splat(vec.x), y: Simd::splat(vec.y), z: Simd::splat(vec.z) }
    }
    pub fn into_parts(self) -> (Simd<HordeFloat, LANE_COUNT>, Simd<HordeFloat, LANE_COUNT>, Simd<HordeFloat, LANE_COUNT>) {
        (
            self.x,
            self.y,
            self.z
        )
    }
    pub fn det2D(&self, other:&SIMDVec3Df) -> Simd<HordeFloat, LANE_COUNT> {
        self.x * other.y - self.y * other.x
    }
}

impl Sub<Self> for SIMDVec3Df {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub<&Self> for SIMDVec3Df {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Add<Self> for SIMDVec3Df {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<&Self> for SIMDVec3Df {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}


pub struct SIMDVec2Df {
    pub x:Simd<HordeFloat, LANE_COUNT>,
    pub y:Simd<HordeFloat, LANE_COUNT>,
}

impl SIMDVec2Df {
    pub fn new(x:Simd<HordeFloat, LANE_COUNT>, y:Simd<HordeFloat, LANE_COUNT>) -> Self {
        Self { x, y }
    }
    pub fn from_vec3D(vec:&Vec3Df) -> Self {
        Self { x: Simd::splat(vec.x), y: Simd::splat(vec.y)}
    }
    pub fn det(&self, other:&SIMDVec2Df) -> Simd<HordeFloat, LANE_COUNT> {
        self.x * other.y - self.y * other.x
    }
}