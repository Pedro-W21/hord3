pub mod framebuffer;
pub mod camera;

pub trait RenderingBackend: Sized + Sync + Send + Clone {
    type RenderingStatusUpdate;
    type RenderingStatus;
    type EntityRenderingData;
    type PreTickData;
}