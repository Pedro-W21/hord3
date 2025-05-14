use crate::horde::geometry::{rotation::Orientation, vec3d::Vec3Df};

#[derive(Clone, Debug)]
pub struct Camera {
    pub pos:Vec3Df,
    pub orient:Orientation,
    pub fov:f32
}

impl Camera {
    pub fn empty() -> Self {
        Self { pos: Vec3Df::zero(), orient: Orientation::zero(), fov:90.0 }
    }
    pub fn new(pos:Vec3Df, orient:Orientation) -> Self {
        Self { pos, orient, fov:90.0 }
    }
}