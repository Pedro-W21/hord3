use std::{ops::Deref, simd::{Simd, SupportedLaneCount}};

use crate::horde::geometry::{plane::EquationPlane, rotation::Rotation, vec3d::Vec3Df, HordeFloat};

#[derive(Clone, Copy, Debug)]
pub struct Vec3DfCam(pub Vec3Df);

impl Deref for Vec3DfCam {
    type Target = Vec3Df;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Vec3DfCam {
    pub fn from_realspace(realspace:Vec3Df, campos:&Vec3Df, rotation:&Rotation) -> Self {
        Self(rotation.rotate(realspace - campos))
    }
}
#[derive(Clone, Copy)]
pub struct Vec3DfRaster(pub Vec3Df);

impl Deref for Vec3DfRaster {
    type Target = Vec3Df;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Vec3DfRaster {
    pub fn from_cameraspace(cameraspace:Vec3DfCam, viewport_data:&ViewportData) -> Self {
        let z = 1.0 / cameraspace.z;//(pos_finale.x.abs().clamp(0.001, INFINITY)).copysign(pos_finale.x);
        Vec3DfRaster(Vec3Df::new(
            (1.0 + (viewport_data.near_clipping_plane * cameraspace.x * z)) * viewport_data.half_image_width,
            (1.0 - (viewport_data.near_clipping_plane * cameraspace.y * z))
                * viewport_data.half_image_height
                * viewport_data.aspect_ratio,
            z,
        ))
    }
    pub fn is_point_on_screen(&self, viewport_data:&ViewportData) -> bool {
        self.x >= 0.0 && self.x < viewport_data.image_width && self.y >= 0.0 && self.y < viewport_data.image_height
    }
}


#[derive(Clone)]
pub struct ViewportData {
    pub near_clipping_plane:HordeFloat,
    pub half_image_width:HordeFloat,
    pub half_image_height:HordeFloat,
    pub aspect_ratio:HordeFloat,
    pub camera_plane:EquationPlane,
    pub image_height:HordeFloat,
    pub image_width:HordeFloat,
    pub poscam:Vec3Df,
    pub rotat_cam:Rotation
}