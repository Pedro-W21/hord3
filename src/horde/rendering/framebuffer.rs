pub trait FrameBuffer {
    type FBD:FrameBufferDimensions;
    fn get_format(&self) -> HordeColorFormat;
    fn get_raw(&self) -> Vec<u8>;
    fn get_dimensions(&self) -> Self::FBD;
    fn copy_into<FB2:FrameBuffer>(&mut self, target:&mut FB2);
}

pub trait FrameBufferDimensions {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
    fn get_aspect_ratio(&self) -> f32;
}

#[derive(Clone, Copy)]
pub enum HordeColorFormat {
    RGB888,
    ARGB8888,
    RGBA8888,
}