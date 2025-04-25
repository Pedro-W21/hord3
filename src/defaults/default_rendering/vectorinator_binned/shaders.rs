use crate::horde::{geometry::rotation::Rotation, rendering::camera::Camera};

pub trait ShaderData:Clone + Sync + Send {
    type SFD:ShaderFrameData;
    fn get_frame_data(&self, cam:&Camera, rotat_cam:&Rotation) -> Self::SFD;
    fn get_raw_frame_data(&self) -> Self::SFD;
}

pub trait ShaderFrameData {
    fn get_new_pixel(&mut self, pixel_index:usize, old_color:u32, old_depth:f32, old_normal:u32, framebuf:&Vec<u32>, zbuf:&Vec<f32>, nbuf:&Vec<u32>, width:usize, height:usize) -> u32;
}

#[derive(Clone)]
pub struct NoOpShader {}
impl ShaderData for NoOpShader {
    type SFD = NoOpShader;
    fn get_frame_data(&self, cam:&Camera, rotat_cam:&Rotation) -> Self::SFD {
        self.clone()
    }
    fn get_raw_frame_data(&self) -> Self::SFD {
        self.clone()
    }
}
impl ShaderFrameData for NoOpShader {
    fn get_new_pixel(&mut self, pixel_index:usize, old_color:u32, old_depth:f32, old_normal:u32, framebuf:&Vec<u32>, zbuf:&Vec<f32>, nbuf:&Vec<u32>, width:usize, height:usize) -> u32 {
        old_color
    }
}